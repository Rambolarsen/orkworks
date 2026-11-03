use super::child_sources::*;
use super::coordinator::{canonical_json_bytes, sha256_hex};
use serde_json::{json, Value};
fn catalog() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/validation/child-source-vectors.json"
    ))
    .unwrap()
}
fn canon(v: &Value) -> Vec<u8> {
    canonical_json_bytes(v).unwrap()
}
struct OwnedBundle {
    records: Vec<Vec<u8>>,
    evidence: Vec<Vec<u8>>,
}
const FIELDS: [&str; 9] = [
    "configuration",
    "plan",
    "request",
    "approval",
    "receipt",
    "reservation",
    "allocation",
    "attachment",
    "source",
];
impl OwnedBundle {
    fn new(p: &Value) -> Self {
        Self {
            records: FIELDS.iter().map(|k| canon(&p[k])).collect(),
            evidence: p["evidenceObjects"]
                .as_array()
                .unwrap()
                .iter()
                .map(canon)
                .collect(),
        }
    }
    fn input<'a>(&'a self, evidence: &'a [&'a [u8]]) -> ChildSourceBundle<'a> {
        ChildSourceBundle {
            configuration: &self.records[0],
            evidence_objects: evidence,
            plan: &self.records[1],
            request: &self.records[2],
            approval: &self.records[3],
            receipt: &self.records[4],
            reservation: &self.records[5],
            allocation: &self.records[6],
            attachment: &self.records[7],
            source: &self.records[8],
        }
    }
}
#[derive(Debug)]
struct Output {
    class: &'static str,
    configurations: [Vec<u8>; 2],
    sources: [Vec<u8>; 2],
    config_digests: [String; 2],
    source_digests: [String; 2],
}
fn validate(p: &Value, override_wire: Option<(&str, Vec<u8>)>) -> Result<Output, ChildSourceError> {
    let mut worker = OwnedBundle::new(&p["worker"]);
    let mut reviewer = OwnedBundle::new(&p["reviewer"]);
    let mut run = canon(&p["run"]);
    let mut parent = canon(&p["parentInput"]);
    let mut snapshot = canon(&p["snapshot"]);
    if let Some((field, bytes)) = override_wire {
        match field {
            "run" => run = bytes,
            "parentInput" => parent = bytes,
            "snapshot" => snapshot = bytes,
            _ => {
                let (owner, field) = if let Some(field) = field.strip_prefix("reviewer.") {
                    (&mut reviewer, field)
                } else {
                    (&mut worker, field)
                };
                if field == "evidence" {
                    owner.evidence[0] = bytes;
                } else {
                    owner.records[FIELDS.iter().position(|k| *k == field).unwrap()] = bytes;
                }
            }
        }
    }
    let we: Vec<_> = worker.evidence.iter().map(Vec::as_slice).collect();
    let re: Vec<_> = reviewer.evidence.iter().map(Vec::as_slice).collect();
    let input = ChildSourceValidationInput {
        run: &run,
        parent_input: &parent,
        worker: worker.input(&we),
        reviewer: reviewer.input(&re),
        snapshot: &snapshot,
        report_context: &p["reportContext"],
    };
    let out = validate_unverified_child_sources_v1(&input)?;
    Ok(Output {
        class: out.classification(),
        configurations: [
            out.worker().configuration_bytes().to_vec(),
            out.reviewer().configuration_bytes().to_vec(),
        ],
        sources: [
            out.worker().source_bytes().to_vec(),
            out.reviewer().source_bytes().to_vec(),
        ],
        config_digests: [
            out.worker().configuration_digest().into(),
            out.reviewer().configuration_digest().into(),
        ],
        source_digests: [
            out.worker().source_digest().into(),
            out.reviewer().source_digest().into(),
        ],
    })
}
#[test]
fn child_sources_accept_independent_source_pair_without_promoting_support() {
    let p = catalog();
    let out = validate(&p, None).unwrap();
    assert_eq!(out.class, "unverified-source-bytes-only");
    for (i, name) in ["worker", "reviewer"].iter().enumerate() {
        for (kind, bytes) in [
            ("configuration", &out.configurations[i]),
            ("source", &out.sources[i]),
        ] {
            let v = p["vectors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == format!("{name}-{kind}"))
                .unwrap();
            assert_eq!(bytes, v["canonical"].as_str().unwrap().as_bytes());
            assert_eq!(sha256_hex(bytes), v["sha256"].as_str().unwrap());
        }
        assert_eq!(
            out.config_digests[i],
            p[name]["configuration"]["configurationDigest"]
                .as_str()
                .unwrap()
        );
        assert_eq!(
            out.source_digests[i],
            p["vectors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == format!("{name}-source"))
                .unwrap()["domainDigest"]
                .as_str()
                .unwrap()
        );
    }
}
#[test]
fn child_sources_reject_noncanonical_and_malformed_wire() {
    let p = catalog();
    for bytes in [
        b"{\"a\":1,\"a\":2}".to_vec(),
        b"{\"a\":1,\"\\u0061\":2}".to_vec(),
        b"{\"a\":1.5}".to_vec(),
        b"{\"a\":1e3}".to_vec(),
        b"{\"a\":-0}".to_vec(),
        b"{\"a\":9007199254740993}".to_vec(),
        b"{\"a\":\"\\ud800\"}".to_vec(),
        b"{\"b\":1,\"a\":2}".to_vec(),
        b"{ \"a\":1}".to_vec(),
        vec![0xff],
    ] {
        assert!(
            validate(&p, Some(("configuration", bytes.clone()))).is_err(),
            "{bytes:?}"
        );
    }
}
#[test]
fn child_sources_reject_closed_shapes_and_original_identity_drift() {
    let cases: Vec<(&str, fn(&mut Value))> = vec![
        ("extra field", |p| {
            p["worker"]["configuration"]["grant"] = json!(true)
        }),
        ("root configuration", |p| {
            p["worker"]["configuration"]["schemaVersion"] = json!(2)
        }),
        ("source active flag", |p| {
            p["worker"]["source"]["active"] = json!(true)
        }),
        ("frozen identity", |p| {
            p["worker"]["attachment"]["assignmentIdentity"]["taskVersion"] = json!(3)
        }),
        ("worker and reviewer same child", |p| {
            p["reviewer"]["reservation"]["childSessionId"] =
                p["worker"]["reservation"]["childSessionId"].clone()
        }),
    ];
    for (name, mutate) in cases {
        let mut p = catalog();
        mutate(&mut p);
        assert!(validate(&p, None).is_err(), "{name}");
    }
}
#[test]
fn child_sources_match_independent_129_case_assay() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("node")
        .arg(root.join("scripts/child-source-parity-cases.mjs"))
        .output()
        .expect("Node test bridge");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let corpus: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(corpus["tests"], 129);
    let cases = corpus["cases"].as_array().unwrap();
    assert!(cases.len() > 100);
    for case in cases {
        let result = validate(&case["packet"], None);
        assert_eq!(
            result.is_ok(),
            case["accepted"].as_bool().unwrap(),
            "{}: {result:?}",
            case["name"]
        );
    }
}
#[test]
fn child_sources_pin_all_independent_vector_bytes_and_domains() {
    let p = catalog();
    let vectors = p["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 28);
    for v in vectors {
        let bytes = v["canonical"].as_str().unwrap().as_bytes();
        let value: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(canon(&value), bytes, "{}", v["id"]);
        assert_eq!(bytes.len(), v["bytes"].as_u64().unwrap() as usize);
        assert_eq!(sha256_hex(bytes), v["sha256"].as_str().unwrap());
        if let Some(expected) = v["domainDigest"].as_str() {
            let domain = match v["kind"].as_str().unwrap() {
                "source" => "orkworks.child-assignment-source.v1",
                "evidence-object" => "orkworks.child-source-evidence.v1",
                "request" => "orkworks.child-source-plan-request.v1",
                "approval" => "orkworks.child-source-plan-approval.v1",
                "result" => "orkworks.assignment-result.v1",
                _ => panic!("unknown whole-body domain"),
            };
            assert_eq!(
                sha256_hex(&[domain.as_bytes(), b"\n", bytes].concat()),
                expected
            );
        }
    }
}
#[test]
fn child_sources_bound_every_raw_input_before_semantic_validation() {
    let p = catalog();
    for (field, max) in [
        ("run", 16384),
        ("parentInput", 32768),
        ("snapshot", 262144),
        ("configuration", 1048576),
        ("plan", 65536),
        ("request", 16384),
        ("approval", 16384),
        ("receipt", 4096),
        ("reservation", 16384),
        ("allocation", 16384),
        ("attachment", 16384),
        ("source", 2097152),
        ("evidence", 2097152),
    ] {
        let oversized = vec![b' '; max + 1];
        assert!(validate(&p, Some((field, oversized))).is_err(), "{field}");
        for field in if ["run", "parentInput", "snapshot"].contains(&field) {
            vec![field.to_owned()]
        } else {
            vec![field.to_owned(), format!("reviewer.{field}")]
        } {
            assert_eq!(
                validate(&p, Some((&field, b"{\"a\":1,\"a\":2}".to_vec()))).unwrap_err(),
                ChildSourceError("canonical wire"),
                "{field}"
            );
        }
    }
}
#[test]
fn child_sources_consume_each_independent_vector_in_its_complete_context() {
    let original = catalog();
    for v in original["vectors"].as_array().unwrap() {
        let mut p = original.clone();
        let value: Value = serde_json::from_str(v["canonical"].as_str().unwrap()).unwrap();
        let name = v["id"].as_str().unwrap();
        let kind = v["kind"].as_str().unwrap();
        if kind == "review-snapshot" {
            p["snapshot"] = value;
        } else if kind == "result" {
            p["snapshot"]["result"] = value;
        } else {
            let owner = if name.starts_with("worker-") {
                &mut p["worker"]
            } else {
                &mut p["reviewer"]
            };
            if kind == "evidence-object" {
                let index: usize = name.rsplit('-').next().unwrap().parse().unwrap();
                owner["evidenceObjects"][index] = value;
            } else {
                owner[kind] = value;
            }
        }
        let output = validate(&p, None).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(output.class, "unverified-source-bytes-only");
        for (i, owner) in ["worker", "reviewer"].iter().enumerate() {
            assert_eq!(
                output.configurations[i],
                canon(&original[owner]["configuration"])
            );
            assert_eq!(output.sources[i], canon(&original[owner]["source"]));
        }
    }
}
