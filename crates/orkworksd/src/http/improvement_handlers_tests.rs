use super::*;

#[tokio::test]
async fn improvement_routes_are_authorized_strict_and_remain_unavailable() {
    let _token = crate::test_support::open_plan_token::set("improvement-test-token").await;
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    assert_eq!(
        status(State(state.clone()), HeaderMap::new())
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-orkworks-open-plan-token",
        "improvement-test-token".parse().unwrap(),
    );
    assert_eq!(
        start(
            State(state.clone()),
            headers.clone(),
            Bytes::from_static(b"{}")
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let response = start(State(state.clone()), headers.clone(), Bytes::new()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
        br#"{"outcome":"unavailable","runId":null,"reasonCode":"qualification_disabled"}"#[..]
    );
    assert_eq!(
        cancel(
            State(state.clone()),
            headers.clone(),
            Bytes::from_static(br#"{"runId":"bad/id"}"#)
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let response = cancel(
        State(state.clone()),
        headers.clone(),
        Bytes::from_static(br#"{"runId":"missing"}"#),
    )
    .await;
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        body,
        serde_json::json!({"outcome":"not_running","runId":"missing","reasonCode":null})
    );
    let response = status(State(state), headers).await;
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["available"], false);
    assert_eq!(body["reasonCode"], "qualification_disabled");
}

#[tokio::test]
async fn improvement_router_rejects_oversized_start_and_cancel_before_dispatch() {
    let _token = crate::test_support::open_plan_token::set("improvement-limit-token").await;
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let app = crate::build_router(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::new();
    for path in ["/taskmaster/improvement", "/taskmaster/improvement/cancel"] {
        let response = client
            .post(format!("http://{address}{path}"))
            .header("x-orkworks-open-plan-token", "improvement-limit-token")
            .body("x".repeat(1025))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 413, "{path}");
    }
    server.abort();
}
