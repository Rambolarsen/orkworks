//! Negotiated presentation for validated producer data; never a policy selector.
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum View {
    Legacy,
    V3,
}

pub(crate) fn upgrade_required() -> Response {
    (
        StatusCode::UPGRADE_REQUIRED,
        Json(json!({"reasonCode":"protocol_upgrade_required"})),
    )
        .into_response()
}

// These discriminators are supplied only by validated server codecs/projections.
// A submitted or generated verificationStatus is never a producer discriminator.
fn is_v3(value: &Value) -> bool {
    value.get("producerVersion") == Some(&json!(3))
        || value.get("improvementVersion") == Some(&json!(3))
        || value.pointer("/assessment/improvementReference/version") == Some(&json!(3))
}

impl View {
    pub(crate) fn application(
        self,
        state: std::sync::Arc<crate::AppState>,
    ) -> crate::session_application::SessionApplication {
        let application = crate::session_application::SessionApplication::new(state);
        if self == Self::V3 {
            application.with_unverified_advice_view()
        } else {
            application
        }
    }
    pub(crate) fn parse(headers: &HeaderMap) -> Result<Self, Response> {
        let mut declarations = headers.get_all("x-orkworks-advice-protocol").iter();
        match (declarations.next(), declarations.next()) {
            (None, None) => Ok(Self::Legacy),
            (Some(value), None) if value.as_bytes() == b"3" => Ok(Self::V3),
            _ => Err(StatusCode::BAD_REQUEST.into_response()),
        }
    }

    pub(crate) fn require_v3(self) -> Result<(), Response> {
        if self == Self::V3 {
            Ok(())
        } else {
            Err(upgrade_required())
        }
    }

    pub(crate) fn require_record(
        self,
        item: &crate::taskmaster::Recommendation,
    ) -> Result<(), Response> {
        if item
            .assessment
            .as_ref()
            .and_then(|a| a.improvement_reference.as_ref())
            .is_some_and(|r| r.version == 3)
        {
            self.require_v3()
        } else {
            Ok(())
        }
    }

    /// Acknowledgement is lifecycle data. Fetch advice separately through its
    /// currentness/privacy fence; never echo captured prose after an action.
    pub(crate) fn mutation(self, item: &crate::taskmaster::Recommendation) -> Response {
        if crate::taskmaster::is_unverified_improvement(item) {
            if let Err(response) = self.require_v3() {
                return response;
            }
            self.json(
                json!({"id":item.id,"status":item.status,"targetSessionId":item.target_session_id}),
            )
        } else {
            self.json(item)
        }
    }

    pub(crate) fn json(self, value: impl Serialize) -> Response {
        let Ok(value) = serde_json::to_value(value) else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        let Some(mut value) = self.project(value) else {
            return upgrade_required();
        };
        if self == Self::V3 {
            if let Some(object) = value.as_object_mut() {
                object.insert("protocolVersion".into(), json!(3));
            }
        }
        Json(value).into_response()
    }

    fn project(self, value: Value) -> Option<Value> {
        let new_producer = is_v3(&value);
        if new_producer && self == Self::Legacy {
            return None;
        }
        match value {
            Value::Array(items) => Some(Value::Array(
                items
                    .into_iter()
                    .filter_map(|item| self.project(item))
                    .collect(),
            )),
            Value::Object(mut object) => {
                let hidden_latest = self == Self::Legacy
                    && object
                        .get("snapshots")
                        .and_then(Value::as_array)
                        .and_then(|items| items.first())
                        .is_some_and(is_v3);
                let hidden_report = self == Self::Legacy
                    && object
                        .get("run")
                        .and_then(|run| run.get("report"))
                        .is_some_and(is_v3);
                let hidden_recommendation =
                    self == Self::Legacy && object.get("recommendation").is_some_and(is_v3);
                for value in object.values_mut() {
                    *value = self.project(std::mem::take(value)).unwrap_or(Value::Null);
                }
                object.remove("verificationStatus");
                if new_producer {
                    object.insert("protocolVersion".into(), json!(3));
                    object.insert("verificationStatus".into(), json!("unverified"));
                }
                if hidden_latest {
                    object.insert("freshness".into(), json!("unknown"));
                }
                if hidden_report {
                    object.insert("available".into(), json!(false));
                    object.insert("reasonCode".into(), json!("protocol_upgrade_required"));
                }
                if hidden_recommendation {
                    object.insert("recoveryAllowed".into(), json!(false));
                    object.insert("reasonCode".into(), json!("protocol_upgrade_required"));
                }
                Some(Value::Object(object))
            }
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use serde_json::json;

    #[test]
    fn declaration_is_absent_or_exactly_one_supported_value() {
        let mut headers = HeaderMap::new();
        assert!(matches!(View::parse(&headers), Ok(View::Legacy)));
        headers.insert("x-orkworks-advice-protocol", "3".parse().unwrap());
        assert!(matches!(View::parse(&headers), Ok(View::V3)));
        headers.append("x-orkworks-advice-protocol", "3".parse().unwrap());
        assert!(View::parse(&headers).is_err());
        for value in ["2", "4", "03", "3, 3"] {
            headers.insert("x-orkworks-advice-protocol", value.parse().unwrap());
            assert!(View::parse(&headers).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn legacy_views_omit_new_prose_even_when_nested_and_do_not_invent_freshness() {
        let value = json!({"freshness":"current","snapshots":[
            {"id":"new","improvementVersion":3,"data":{"summary":"Tests passed."}},
            {"id":"old","data":{"summary":"Old advice"}}
        ],"recommendations":[
            {"id":"new","summary":"Tests passed.","assessment":{"improvementReference":{"version":3}}},
            {"id":"old","summary":"Old advice"}
        ],"recommendation":{"id":"new","summary":"Tests passed.","assessment":{"improvementReference":{"version":3}}}});
        let output = View::Legacy.project(value.clone()).unwrap();
        assert_eq!(output["freshness"], "unknown");
        assert_eq!(output["snapshots"].as_array().unwrap().len(), 1);
        assert_eq!(output["snapshots"][0]["id"], "old");
        assert_eq!(output["recommendations"].as_array().unwrap().len(), 1);
        assert!(output["recommendation"].is_null());
        assert!(!output.to_string().contains("Tests passed."));
        let mut older_hidden = value;
        older_hidden["snapshots"].as_array_mut().unwrap().reverse();
        assert_eq!(
            View::Legacy.project(older_hidden).unwrap()["freshness"],
            "current"
        );
    }

    #[test]
    fn new_view_derives_unverified_status_for_each_producer_unit() {
        for value in [
            json!({"producerVersion":3,"summary":"Tests passed.","verificationStatus":"verified"}),
            json!({"id":"new","improvementVersion":3,"data":null}),
            json!({"id":"new","assessment":{"improvementReference":{"version":3}},"summary":"Tests passed."}),
        ] {
            assert!(View::Legacy.project(value.clone()).is_none());
            let output = View::V3.project(value).unwrap();
            assert_eq!(output["protocolVersion"], 3);
            assert_eq!(output["verificationStatus"], "unverified");
        }
        let output = View::V3
            .project(json!({"verificationStatus":"verified","summary":"Legacy text"}))
            .unwrap();
        assert!(
            output.get("verificationStatus").is_none(),
            "a standalone status is never authority"
        );
    }

    #[test]
    fn legacy_projection_preserves_old_records_and_never_mutates_its_source() {
        let old = json!({"id":"old","summary":"Old advice","assessment":{"improvementReference":{"version":2}}});
        let value = json!({"snapshots":[{"id":"redacted-new","improvementVersion":3,"data":null,"redacted":true}],"recommendations":[old.clone()]});
        let bytes = serde_json::to_vec(&value).unwrap();
        let projected = View::Legacy.project(value.clone()).unwrap();
        assert_eq!(projected["recommendations"][0], old);
        assert!(projected["snapshots"].as_array().unwrap().is_empty());
        assert_eq!(serde_json::to_vec(&value).unwrap(), bytes);
        assert_eq!(View::Legacy.project(old.clone()).unwrap(), old);
    }
}
