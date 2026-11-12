use super::Manifest;
use crate::taskmaster::{assessment_context::improvement::RangeIdentity, runtime::ContextLevel};

#[test]
fn manifest_rejects_oversized_ranges_without_summing_untrusted_sizes() {
    let manifest = Manifest {
        purpose: "brain-improvement-v1".into(),
        workspace_path: "/workspace".into(),
        workspace_instance: 1,
        context_level: ContextLevel::SourceCode,
        excluded_paths: Vec::new(),
        evaluation_key: "0".repeat(64),
        rubric_digest: "0".repeat(64),
        catalog_digest: "0".repeat(64),
        ranges: [0, 1]
            .map(|index| RangeIdentity {
                path: format!("src/{index}.rs"),
                offset: 0,
                bytes: usize::MAX,
                file_size: u64::MAX,
                sha256: "0".repeat(64),
            })
            .into(),
        pages: Vec::new(),
        recommendation_digest: "0".repeat(64),
    };

    assert!(manifest.validate().is_err());
}
