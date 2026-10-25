//! Static shapes for explicitly constrained native Codex assessment calls.
use super::{invalid, ProviderOperationError};
use crate::providers::native_inference::NativeResponseShape;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn prose() -> Value {
    json!({"type":"string", "description":"A single-line declarative description using nouns for execution concerns; avoid the words run, execute, invoke, make, just, pass and successful. Never include literal commands, tool actions, code or claims that checks were performed. Proposed validation stays unverified."})
}
fn references() -> Value {
    json!({"type":"array", "items":{"type":"string"}})
}
fn target_surface() -> Value {
    json!({"type":"string", "enum":["instructions","skill","test","tooling","documentation"]})
}
fn title() -> Value {
    json!({"type":"string", "description":"A short noun phrase naming the workflow concern, not an instruction or executable command."})
}
fn nullable(object: Value) -> Value {
    json!({"anyOf":[object,{"type":"null"}]})
}

fn schema(shape: NativeResponseShape) -> Value {
    match shape {
        NativeResponseShape::ReadinessDiscoveryV1 => {
            json!({"type":"object","additionalProperties":false,"required":["version","requestedFileIds"],"properties":{"version":{"type":"integer","enum":[1]},"requestedFileIds":{"type":"array","items":{"type":"integer"}}}})
        }
        NativeResponseShape::ReadinessV1 => {
            json!({"type":"object","additionalProperties":false,"required":["version","summary","concepts"],"properties":{
                "version":{"type":"integer","enum":[1]},"summary":prose(),"concepts":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["conceptId","maturity","confidence","repositoryFactIds","rubricPageId","rationale","limits","nextTransition","unassessedReason"],"properties":{
                    "conceptId":{"type":"string"},"maturity":nullable(json!({"type":"integer","minimum":0,"maximum":4})),"confidence":nullable(json!({"type":"string","enum":["low","medium","high"]})),"repositoryFactIds":{"type":"array","items":{"type":"integer"}},"rubricPageId":{"type":"string"},"rationale":prose(),"limits":{"type":"array","items":prose()},"nextTransition":prose(),"unassessedReason":nullable(prose())
                }}}
            }})
        }
        NativeResponseShape::DiscoveryV1 => json!({
            "type":"object", "additionalProperties":false,
            "required":["version","outcome","candidate","repositoryFactHashes","knowledgePageIds","requestedFileIds","reason"],
            "properties":{
                "version":{"type":"integer","enum":[1]},
                "outcome":{"type":"string","enum":["inspect","no_candidate"]},
                "candidate":nullable(json!({
                    "type":"object", "additionalProperties":false,
                    "required":["targetSurface","title","hypothesis"],
                    "properties":{"targetSurface":target_surface(),"title":title(),"hypothesis":prose()}
                })),
                "repositoryFactHashes":references(),
                "knowledgePageIds":references(),
                "requestedFileIds":{"type":"array","items":{"type":"integer"}},
                "reason":nullable(prose())
            }
        }),
        NativeResponseShape::AssessmentV1 => json!({
            "type":"object", "additionalProperties":false,
            "required":["version","outcome","summary","repositoryFactHashes","knowledgePageIds","unverifiedChecks","proposal","noProposalReason"],
            "properties":{
                "version":{"type":"integer","enum":[1]},
                "outcome":{"type":"string","enum":["proposed","no_proposal"]},
                "summary":{"type":"string", "description":"A single-line declarative assessment of supplied evidence using nouns for execution concerns; avoid run, execute, invoke, make, just, pass and successful. Describe validation scope with nouns; detailed proposed validation conditions belong in unverifiedChecks. Never include commands, execution instructions or completed-check claims; avoid ambiguous make/just phrasing."},
                "repositoryFactHashes":references(),
                "knowledgePageIds":references(),
                "unverifiedChecks":{"type":"array", "items":{"type":"string", "description":"A proposed validation condition, at most 200 characters. Start with Whether and end with remains unverified. State the condition directly with nouns for execution concerns, avoiding run, execute, invoke, make, just, pass and successful; no instructions or infinitive execution verbs. Never claim checks were performed."}},
                "proposal":nullable(json!({
                    "type":"object", "additionalProperties":false,
                    "required":["targetSurface","title","summary"],
                    "properties":{"targetSurface":target_surface(),"title":title(),"summary":{"type":"string", "description":"Prospective declarative sentences using would describe the bounded target, proposed change, expected benefit, validation scope and rollback. Use nouns for execution concerns, avoiding run, execute, invoke, make, just, pass and successful. Name validation scope with descriptive nouns without completion predicates or execution verbs; detailed proposed validation conditions belong in unverifiedChecks. Never include commands, execution instructions, code or claims that checks were performed; avoid ambiguous make/just phrasing."}}
                })),
                "noProposalReason":nullable(prose())
            }
        }),
    }
}

pub(super) fn write(
    shape: NativeResponseShape,
    directory: &Path,
) -> Result<PathBuf, ProviderOperationError> {
    let bytes = serde_json::to_vec(&schema(shape))
        .map_err(|_| invalid("could not encode native response constraint"))?;
    if bytes.len() > 8192 {
        return Err(invalid(
            "native response constraint exceeds its fixed limit",
        ));
    }
    let path = directory.join("response-schema.json");
    fs::write(&path, bytes).map_err(|_| invalid("could not prepare native response constraint"))?;
    Ok(path)
}
