use super::*;
use crate::taskmaster::runtime::ContextLevel;

#[test]
fn saved_manifest_revalidates_every_range_catalog_guidance_and_recommendation_input() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("AGENTS.md"), "captured instructions").unwrap();
    std::fs::create_dir(workspace.path().join("src")).unwrap();
    let late_path = workspace.path().join("src/late.rs");
    std::fs::write(&late_path, format!("{}later evidence", "a".repeat(9000))).unwrap();
    let mut evidence =
        ProgressiveEvidence::collect(workspace.path(), ContextLevel::SourceCode, &[], "now")
            .unwrap();
    let file_id = evidence
        .catalog()
        .iter()
        .position(|path| path == "src/late.rs")
        .unwrap();
    evidence.read(file_id, 9000, 14).unwrap();
    let pages = vec![PageIdentity {
        id: "page".into(),
        sha256: "a".repeat(64),
    }];
    let manifest = Manifest {
        purpose: "brain-improvement-v1".into(),
        workspace_path: workspace.path().to_str().unwrap().into(),
        workspace_instance: 1,
        context_level: ContextLevel::SourceCode,
        excluded_paths: vec![],
        evaluation_key: "b".repeat(64),
        rubric_digest: "c".repeat(64),
        catalog_digest: evidence.catalog_digest().into(),
        ranges: evidence.range_manifest(),
        pages: pages.clone(),
        recommendation_digest: "d".repeat(64),
    };
    let check = |pages: &[PageIdentity], digest: &str| {
        verify_inputs(
            &manifest,
            workspace.path(),
            ContextLevel::SourceCode,
            &[],
            &"b".repeat(64),
            &"c".repeat(64),
            pages,
            digest,
        )
    };
    assert!(check(&pages, &"d".repeat(64)).is_ok());
    assert!(check(&pages, &"e".repeat(64)).is_err());
    assert!(check(&[], &"d".repeat(64)).is_err());
    std::fs::write(&late_path, format!("{}changed proof!", "a".repeat(9000))).unwrap();
    assert!(check(&pages, &"d".repeat(64)).is_err());
}
