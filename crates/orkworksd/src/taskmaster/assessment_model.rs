use crate::taskmaster::TargetSurface;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_TITLE_CHARS: usize = 240;
const MAX_TEXT_CHARS: usize = 1_000;
const MAX_CHECK_CHARS: usize = 200;
const MAX_FACT_HASHES: usize = 32;
const MAX_PAGE_IDS: usize = 8;
const MAX_UNVERIFIED_CHECKS: usize = 4;
const ENVELOPE_FIELDS: [&str; 8] = [
    "version",
    "outcome",
    "summary",
    "repositoryFactHashes",
    "knowledgePageIds",
    "unverifiedChecks",
    "proposal",
    "noProposalReason",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentResponse {
    pub version: u8,
    pub outcome: AssessmentOutcome,
    pub summary: String,
    pub repository_fact_hashes: Vec<String>,
    pub knowledge_page_ids: Vec<String>,
    pub unverified_checks: Vec<String>,
    pub proposal: Option<AssessmentProposal>,
    pub no_proposal_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssessmentOutcome {
    Proposed,
    NoProposal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AssessmentProposal {
    pub target_surface: TargetSurface,
    pub title: String,
    pub summary: String,
}

pub(crate) fn parse_assessment_response(
    output: &str,
    facts: &[crate::taskmaster::RepositoryEvidence],
    pages: &[crate::taskmaster::runtime::KnowledgePage],
) -> Result<AssessmentResponse, String> {
    if output.len() > MAX_RESPONSE_BYTES {
        return Err("assessment response exceeds the size limit".into());
    }
    let envelope: serde_json::Value =
        serde_json::from_str(output).map_err(|_| "assessment response is not valid JSON")?;
    let object = envelope
        .as_object()
        .ok_or("assessment response must be an object")?;
    if ENVELOPE_FIELDS
        .iter()
        .any(|field| !object.contains_key(*field))
    {
        return Err("assessment response is missing a required field".into());
    }

    // Deserialize the original text as well as checking the object keys above:
    // serde reports duplicate keys, while the Value parse gives us presence of
    // nullable keys (which derived Option fields otherwise treat as optional).
    let response: AssessmentResponse =
        serde_json::from_str(output).map_err(|_| "assessment response has an invalid shape")?;
    if response.version != 1 {
        return Err("unsupported assessment response version".into());
    }
    validate_prose(&response.summary, MAX_TEXT_CHARS, true)?;
    validate_unique_bounded(
        &response.repository_fact_hashes,
        MAX_FACT_HASHES,
        "repository hashes",
    )?;
    validate_unique_bounded(
        &response.knowledge_page_ids,
        MAX_PAGE_IDS,
        "knowledge page IDs",
    )?;
    validate_unique_bounded(
        &response.unverified_checks,
        MAX_UNVERIFIED_CHECKS,
        "unverified checks",
    )?;
    for check in &response.unverified_checks {
        validate_prospective_prose(check, MAX_CHECK_CHARS, true)?;
    }

    let known_hashes: HashSet<&str> = facts.iter().map(|fact| fact.sha256.as_str()).collect();
    if response
        .repository_fact_hashes
        .iter()
        .any(|hash| !known_hashes.contains(hash.as_str()))
    {
        return Err("assessment cites an unknown repository fact".into());
    }
    let known_page_ids: HashSet<&str> = pages.iter().map(|page| page.id.as_str()).collect();
    if response
        .knowledge_page_ids
        .iter()
        .any(|page_id| !known_page_ids.contains(page_id.as_str()))
    {
        return Err("assessment cites an unknown knowledge page".into());
    }

    match response.outcome {
        AssessmentOutcome::Proposed => {
            let proposal = response
                .proposal
                .as_ref()
                .ok_or("proposed assessment requires a proposal")?;
            if response.repository_fact_hashes.is_empty() || response.knowledge_page_ids.is_empty()
            {
                return Err(
                    "proposed assessment requires repository and knowledge citations".into(),
                );
            }
            if response.no_proposal_reason.is_some() {
                return Err("proposed assessment cannot include a no-proposal reason".into());
            }
            validate_prose(&proposal.title, MAX_TITLE_CHARS, true)?;
            validate_prose(&proposal.summary, MAX_TEXT_CHARS, true)?;
        }
        AssessmentOutcome::NoProposal => {
            if response.proposal.is_some() {
                return Err("no-proposal assessment cannot include a proposal".into());
            }
            let reason = response
                .no_proposal_reason
                .as_deref()
                .ok_or("no-proposal assessment requires a reason")?;
            validate_prose(reason, MAX_TEXT_CHARS, true)?;
        }
    }

    Ok(response)
}

fn validate_unique_bounded(values: &[String], limit: usize, label: &str) -> Result<(), String> {
    if values.len() > limit {
        return Err(format!("too many {label}"));
    }
    let mut seen = HashSet::with_capacity(values.len());
    if values.iter().any(|value| !seen.insert(value.as_str())) {
        return Err(format!("duplicate {label}"));
    }
    if values
        .iter()
        .any(|value| value.trim().is_empty() || has_control(value))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

pub(super) fn validate_prose(value: &str, limit: usize, required: bool) -> Result<(), String> {
    validate_prose_for(value, limit, required, false)
}

/// Future checks may name desired evidence without asserting that it exists.
pub(super) fn validate_prospective_prose(
    value: &str,
    limit: usize,
    required: bool,
) -> Result<(), String> {
    validate_prose_for(value, limit, required, true)
}

fn validate_prose_for(
    value: &str,
    limit: usize,
    required: bool,
    prospective: bool,
) -> Result<(), String> {
    let char_count = value.chars().count();
    if char_count > limit
        || has_control(value)
        || (required && value.trim().is_empty())
        || contains_command_or_verification_claim(value, prospective)
    {
        return Err("assessment contains unsafe or out-of-bounds text".into());
    }
    Ok(())
}

fn has_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn contains_command_or_verification_claim(value: &str, prospective: bool) -> bool {
    let lower = value.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    if lower.contains("&&") || lower.contains("||") || lower.contains("$(") || lower.contains("${")
    {
        return true;
    }
    let shell_operator = lower.contains('|') || lower.contains('>') || lower.contains('<');
    // A semicolon also joins ordinary prose. Only capitalized platform
    // descriptions below disambiguate Node from an executable name. All other
    // bare command names remain conservative, including capitalized spellings.
    let semicolon_command = lower.contains(';') && contains_command_token(&words) && {
        let original_words: Vec<&str> = value.split_whitespace().collect();
        let bare_command = original_words.iter().enumerate().any(|(index, word)| {
            let next = original_words
                .get(index + 1)
                .map(|word| word.trim_end_matches(['.', ',', ':', ';', '!', '?']));
            let node_platform_description = *word == "Node"
                && (next.is_some_and(|word| {
                    matches!(
                        word,
                        "setup" | "version" | "versions" | "runtime" | "requirements" | "toolchain"
                    )
                }) || next.is_some_and(|word| {
                    word.chars().any(|ch| ch.is_ascii_digit())
                        && word.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
                }) || (next == Some("and")
                    && original_words.get(index + 2) == Some(&"pnpm")
                    && original_words.get(index + 3).is_some_and(|word| {
                        matches!(
                            word.trim_end_matches(['.', ',', ':', ';']),
                            "versions" | "requirements"
                        )
                    }))
                    || (next == Some("package")
                        && original_words.get(index + 2).is_some_and(|word| {
                            matches!(
                                word.trim_end_matches(['.', ',', ':', ';', '!', '?']),
                                "management" | "configuration"
                            )
                        })));
            let lower_word = word.to_ascii_lowercase();
            !node_platform_description
                && lower_word
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .any(|token| contains_command_token(&[token]))
        });
        bare_command || contains_embedded_literal_command(&lower)
    };
    if contains_imperative_command(&lower)
        || contains_tool_invocation(value, &words)
        || contains_direct_command(value)
        || (shell_operator && contains_command_token(&words))
        || semicolon_command
    {
        return true;
    }
    has_completed_verification_claim(&lower, prospective)
}

fn contains_command_token(words: &[&str]) -> bool {
    const COMMANDS: [&str; 19] = [
        "bash", "sh", "python", "node", "rm", "mkdir", "cd", "curl", "wget", "printf", "echo",
        "cat", "ls", "grep", "sed", "awk", "touch", "chmod", "find",
    ];
    words.iter().any(|word| COMMANDS.contains(word))
}

fn contains_direct_command(text: &str) -> bool {
    const COMMANDS: [&str; 24] = [
        "bash", "sh", "python", "node", "rm", "mkdir", "cd", "curl", "wget", "printf", "echo",
        "cat", "ls", "grep", "sed", "awk", "touch", "chmod", "find", "tar", "docker", "pytest",
        "make", "just",
    ];
    text.split(['!', '?', ':', ';', '|', '`']).any(|segment| {
        let mut tokens = segment.split_whitespace();
        let Some(command) = tokens
            .next()
            .map(|word| word.trim_matches(['`', '\"', '\'']))
        else {
            return false;
        };
        if !COMMANDS.contains(&command) || command != command.to_ascii_lowercase() {
            return false;
        }
        let Some(argument) = tokens.next() else {
            return true;
        };
        let argument = argument.trim_matches(['`', '\"', '\'']);
        matches!(
            command,
            "echo" | "mkdir" | "cd" | "rm" | "pytest" | "make" | "just"
        ) || argument.starts_with('-')
            || argument.starts_with('/')
            || argument.starts_with("./")
            || argument.starts_with("../")
            || argument.contains('/')
            || argument.rsplit_once('.').is_some_and(|(stem, ext)| {
                !stem.is_empty()
                    && !ext.is_empty()
                    && ext.chars().all(|ch| ch.is_ascii_alphanumeric())
            })
            || (command == "find" && argument == "." && tokens.any(|token| token.starts_with('-')))
    })
}

fn prose_clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['.', '!', '?', ':', ';', ','])
        .flat_map(|clause| clause.split(" but "))
        .flat_map(|clause| clause.split(" however "))
        .flat_map(|clause| clause.split(" yet "))
}

fn contains_embedded_literal_command(text: &str) -> bool {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    if tokens.iter().any(|token| {
        let quoted = token.contains(['`', '\"'])
            || token.char_indices().any(|(index, character)| {
                character == '\''
                    && (index == 0
                        || token[..index]
                            .chars()
                            .next_back()
                            .is_some_and(|previous| !previous.is_alphanumeric()))
            });
        if quoted {
            return true;
        }
        let token = token.trim_matches(['`', '\"', '\'', '.', ',', ':']);
        matches!(
            token,
            "command" | "commands" | "cli" | "executable" | "invocation"
        ) || token.starts_with('-')
            || token.contains('/')
    }) {
        return true;
    }
    tokens.iter().enumerate().any(|(index, token)| {
        // English make/just constructions retain their existing separate checks.
        !matches!(*token, "make" | "just") && contains_direct_command(&tokens[index..].join(" "))
    })
}

fn contains_imperative_command(text: &str) -> bool {
    prose_clauses(text).any(|sentence| {
        let words = sentence
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        words.windows(2).enumerate().any(|(index, pair)| {
            let imperative_position = index == 0
                || matches!(
                    words[index - 1],
                    "please" | "then" | "and" | "must" | "should" | "can" | "you" | "to"
                );
            if !imperative_position
                || !matches!(
                    pair[0],
                    "run"
                        | "execute"
                        | "invoke"
                        | "use"
                        | "call"
                        | "launch"
                        | "start"
                        | "try"
                        | "issue"
                        | "type"
                        | "enter"
                )
            {
                return false;
            }
            if pair[0] == "run" && pair[1] == "through" {
                return false;
            }
            // "Use of ..." is a noun phrase. Prepositional usage with a
            // following predicate is a subject, not an instruction to invoke a tool.
            let nominal_use = pair[0] == "use"
                && (pair[1] == "of"
                    || (matches!(
                        pair[1],
                        "across" | "in" | "within" | "throughout" | "during"
                    ) && words[index + 2..].iter().any(|word| {
                        matches!(*word, "is" | "are" | "was" | "were" | "remains" | "remain")
                    })));
            if nominal_use && !contains_embedded_literal_command(sentence) {
                return false;
            }

            // A configuration description or an indirect question is not an
            // instruction to the reader. Explicit tool syntax is checked separately.
            let addressed_reader = words[..index].iter().any(|word| {
                matches!(
                    *word,
                    "you" | "your" | "we" | "our" | "i" | "reader" | "user" | "agent" | "assistant"
                )
            });
            let repository_subject = words[..index].iter().any(|word| {
                matches!(
                    *word,
                    "ci" | "configuration"
                        | "config"
                        | "script"
                        | "step"
                        | "suite"
                        | "job"
                        | "workflow"
                        | "setup"
                        | "tooling"
                        | "runner"
                )
            });
            let target = if matches!(pair[1], "a" | "an" | "the") {
                words.get(index + 2).copied().unwrap_or("")
            } else {
                pair[1]
            };
            let descriptive_target = matches!(
                target,
                "existing"
                    | "current"
                    | "package"
                    | "unit"
                    | "tests"
                    | "suite"
                    | "checks"
                    | "validation"
                    | "verification"
                    | "with"
                    | "through"
                    | "in"
                    | "on"
                    | "under"
                    | "its"
            ) || (pair[0] == "use"
                && pair[1] == "that"
                && words.get(index + 2) == Some(&"script"));
            if index > 1
                && !addressed_reader
                && descriptive_target
                && !contains_embedded_literal_command(sentence)
                && ((words[index - 1] == "to"
                    && repository_subject
                    && matches!(
                        words[index - 2],
                        "configured"
                            | "designed"
                            | "intended"
                            | "expected"
                            | "scheduled"
                            | "able"
                            | "needed"
                    ))
                    || (words[index - 1] == "can" && words[..index - 1].contains(&"whether")))
            {
                return false;
            }
            // 'Use repository evidence' describes grounding, while 'Use TOOL
            // SUBCOMMAND' is still an executable instruction. Skip an article
            // without exempting the executable that follows it.
            !(pair[0] == "use"
                && matches!(
                    target,
                    "repository"
                        | "evidence"
                        | "guidance"
                        | "context"
                        | "existing"
                        | "supplied"
                        | "relevant"
                        | "current"
                ))
        })
    })
}

fn contains_tool_invocation(text: &str, words: &[&str]) -> bool {
    words
        .windows(2)
        .enumerate()
        .any(|(index, pair)| match pair[0] {
            "cargo" => matches!(
                pair[1],
                "test" | "build" | "check" | "fmt" | "run" | "clippy"
            ),
            "pnpm" | "npm" | "yarn" => matches!(
                pair[1],
                "test" | "build" | "check" | "run" | "install" | "exec" | "dlx"
            ),
            "git" => matches!(
                pair[1],
                "config"
                    | "status"
                    | "diff"
                    | "add"
                    | "commit"
                    | "switch"
                    | "branch"
                    | "checkout"
                    | "log"
                    | "show"
                    | "push"
                    | "pull"
                    | "fetch"
                    | "reset"
                    | "clean"
                    | "worktree"
                    | "merge"
                    | "rebase"
            ),
            "bash" | "sh" | "python" | "node" => {
                pair[1].starts_with('-') || pair[1].contains('.') || pair[1].contains('/')
            }
            // Keep arbitrary recipe names conservative. Exempt grammatical
            // prose constructions, not every unknown make/just target.
            "make" => {
                let descriptive_complement = index == 0
                    && text.trim_start().starts_with("Make ")
                    && words.last().is_some_and(|word| {
                        matches!(
                            *word,
                            "explicit"
                                | "clear"
                                | "clearer"
                                | "easier"
                                | "visible"
                                | "discoverable"
                                | "usable"
                        )
                    });
                !matches!(pair[1], "a" | "an" | "the" | "sure")
                    && !(descriptive_complement
                        && !is_recipe_target(pair[1])
                        && !is_recipe_introduction(words, index)
                        && !contains_embedded_literal_command(&text.to_ascii_lowercase()))
            }
            "just" => {
                let descriptive_adverb = index > 0
                    && matches!(words[index - 1], "is" | "are" | "was" | "were")
                    && !is_recipe_target(pair[1])
                    && !is_recipe_introduction(words, index)
                    && !contains_embedded_literal_command(&text.to_ascii_lowercase());
                !matches!(pair[1], "reports" | "prints" | "emits" | "describes")
                    && !descriptive_adverb
            }
            _ => false,
        })
}

fn is_recipe_introduction(words: &[&str], index: usize) -> bool {
    index >= 2
        && words[index - 1] == "is"
        && matches!(
            words[index - 2],
            "command" | "check" | "step" | "invocation" | "recipe"
        )
}

fn is_recipe_target(word: &str) -> bool {
    matches!(
        word,
        "test"
            | "tests"
            | "build"
            | "check"
            | "lint"
            | "fmt"
            | "install"
            | "clean"
            | "deploy"
            | "release"
            | "run"
    )
}

fn has_completed_verification_claim(text: &str, prospective: bool) -> bool {
    const CHECKS: [&str; 11] = [
        "test",
        "tests",
        "check",
        "checks",
        "build",
        "lint",
        "format",
        "verification",
        "verify",
        "verified",
        "ci",
    ];
    const COMPLETION: [&str; 12] = [
        "pass",
        "passed",
        "passes",
        "passing",
        "succeed",
        "succeeded",
        "successful",
        "successfully",
        "success",
        "verified",
        "green",
        "completed",
    ];
    // A desired evidence fragment is not an exception for an attached claim.
    // Keeping this to one clause also rejects relative assertions like
    // "Evidence of verified tests, which passed" without broadening the
    // completed-claim word list for ordinary repository descriptions.
    let prospective = prospective
        && prose_clauses(text)
            .filter(|clause| !clause.trim().is_empty())
            .count()
            == 1;
    prose_clauses(text).any(|clause| {
        let words = clause
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        words.iter().enumerate().any(|(index, word)| {
            if !COMPLETION.contains(word) {
                return false;
            }
            let start = index.saturating_sub(3);
            let end = (index + 4).min(words.len());
            let nearby_checks = words[start..end]
                .iter()
                .any(|candidate| candidate != word && CHECKS.contains(candidate));
            let before = &words[..index];
            let local = &words[index.saturating_sub(4)..index];
            // A modal qualifies this predicate only through auxiliaries;
            // "can confirm tests passed" still asserts completed verification.
            let modal_predicate = before.iter().enumerate().any(|(offset, word)| {
                matches!(
                    *word,
                    "should" | "would" | "could" | "might" | "may" | "can"
                ) && before[offset + 1..].iter().all(|word| {
                    matches!(
                        *word,
                        "have"
                            | "be"
                            | "been"
                            | "being"
                            | "not"
                            | "never"
                            | "yet"
                            | "ever"
                            | "already"
                            | "still"
                    )
                })
            });
            // Prospective evidence noun phrases describe a desired check,
            // not a completed outcome. Do not exempt a following assertion,
            // such as "Evidence of verified tests is retained" or "tests passed".
            let prospective_modifier = prospective
                && words.starts_with(&["evidence", "of"])
                && matches!(
                    *word,
                    "verified" | "successful" | "completed" | "passing" | "green"
                )
                && words
                    .get(index + 1)
                    .is_some_and(|word| CHECKS.contains(word))
                && words[2..index].iter().all(|word| {
                    matches!(
                        *word,
                        "a" | "an" | "the" | "consistently" | "reliably" | "independently"
                    )
                })
                && !words.iter().any(|word| {
                    matches!(
                        *word,
                        "is" | "are"
                            | "was"
                            | "were"
                            | "has"
                            | "have"
                            | "had"
                            | "shows"
                            | "confirms"
                            | "demonstrates"
                            | "establishes"
                            | "proves"
                            | "exists"
                            | "exist"
                            | "records"
                            | "record"
                            | "confirmed"
                            | "validated"
                            | "demonstrated"
                    )
                });
            let qualified = prospective_modifier
                || before.contains(&"whether")
                || modal_predicate
                || local.iter().any(|word| {
                    matches!(
                        *word,
                        "not"
                            | "never"
                            | "no"
                            | "without"
                            | "reject"
                            | "rejects"
                            | "rejecting"
                            | "avoid"
                            | "avoids"
                            | "avoiding"
                    )
                })
                || (matches!(
                    words.first(),
                    Some(&"validate" | &"confirm" | &"check" | &"determine")
                ) && before.contains(&"that"));
            nearby_checks && !qualified
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::{runtime::KnowledgePage, RepositoryEvidence};
    use serde_json::{json, Value};

    fn facts() -> Vec<RepositoryEvidence> {
        vec![RepositoryEvidence {
            path: "docs/workflow.md".into(),
            sha256: "a".repeat(64),
            excerpt: "A bounded excerpt".into(),
            observed_at: "2026-10-17T10:00:00Z".into(),
        }]
    }

    fn pages() -> Vec<KnowledgePage> {
        vec![KnowledgePage {
            id: "workflow/improvement".into(),
            title: "Workflow improvement".into(),
            page_type: "concept".into(),
            status: "published".into(),
            content: "General workflow guidance".into(),
            sha256: "b".repeat(64),
            related_ids: vec![],
        }]
    }

    fn proposed() -> Value {
        json!({
            "version": 1,
            "outcome": "proposed",
            "summary": "The setup guide omits a repeatable verification step.",
            "repositoryFactHashes": ["a".repeat(64)],
            "knowledgePageIds": ["workflow/improvement"],
            "unverifiedChecks": ["CI configuration should confirm the proposed test command."],
            "proposal": {
                "targetSurface": "documentation",
                "title": "Document the verification workflow",
                "summary": "Record the expected local verification checks."
            },
            "noProposalReason": null
        })
    }

    fn no_proposal() -> Value {
        json!({
            "version": 1,
            "outcome": "no_proposal",
            "summary": "The available excerpts do not show a clear workflow gap.",
            "repositoryFactHashes": [],
            "knowledgePageIds": [],
            "unverifiedChecks": [],
            "proposal": null,
            "noProposalReason": "The bounded evidence is insufficient to support a change."
        })
    }

    fn parse(value: &Value) -> Result<AssessmentResponse, String> {
        parse_assessment_response(&value.to_string(), &facts(), &pages())
    }

    #[test]
    fn accepts_one_grounded_proposal_and_round_trips_exact_wire_names() {
        let result = parse(&proposed()).expect("valid proposal should parse");
        assert_eq!(result.outcome, AssessmentOutcome::Proposed);
        assert_eq!(
            result.proposal.as_ref().unwrap().target_surface,
            TargetSurface::Documentation
        );
        assert!(serde_json::to_string(&result)
            .unwrap()
            .contains("repositoryFactHashes"));
    }

    #[test]
    fn accepts_explicit_no_proposal_without_citations() {
        let result = parse(&no_proposal()).expect("valid no-proposal result should parse");
        assert_eq!(result.outcome, AssessmentOutcome::NoProposal);
        assert!(result.proposal.is_none());
        assert!(result.repository_fact_hashes.is_empty());
    }

    #[test]
    fn requires_nullable_envelope_fields_to_be_present() {
        for key in ["proposal", "noProposalReason"] {
            let mut value = proposed();
            value.as_object_mut().unwrap().remove(key);
            assert!(parse(&value).is_err(), "missing {key} must fail");
        }
    }

    #[test]
    fn rejects_unknown_and_duplicate_fields() {
        let mut unknown = proposed();
        unknown["extra"] = json!(true);
        assert!(parse(&unknown).is_err());

        let duplicate = proposed().to_string();
        let duplicated = duplicate.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
        assert!(parse_assessment_response(&duplicated, &facts(), &pages()).is_err());
    }

    #[test]
    fn rejects_invalid_outcome_combinations_and_multiple_proposals() {
        let mut proposal_missing_refs = proposed();
        proposal_missing_refs["repositoryFactHashes"] = json!([]);
        assert!(parse(&proposal_missing_refs).is_err());

        let mut no_proposal_with_proposal = no_proposal();
        no_proposal_with_proposal["proposal"] = proposed()["proposal"].clone();
        assert!(parse(&no_proposal_with_proposal).is_err());

        let mut too_many = proposed();
        too_many["proposal"] = json!([
            proposed()["proposal"].clone(),
            proposed()["proposal"].clone()
        ]);
        assert!(parse(&too_many).is_err());
    }

    #[test]
    fn rejects_unknown_citations_and_duplicate_references() {
        let mut unknown_hash = proposed();
        unknown_hash["repositoryFactHashes"] = json!(["c".repeat(64)]);
        assert!(parse(&unknown_hash).is_err());

        let mut unknown_page = proposed();
        unknown_page["knowledgePageIds"] = json!(["workflow/missing"]);
        assert!(parse(&unknown_page).is_err());

        let mut duplicate = proposed();
        duplicate["repositoryFactHashes"] = json!(["a".repeat(64), "a".repeat(64)]);
        assert!(parse(&duplicate).is_err());
    }

    #[test]
    fn enforces_response_and_field_bounds_using_unicode_scalars() {
        let mut too_large = proposed();
        too_large["summary"] = json!("界".repeat(1_001));
        assert!(parse(&too_large).is_err());

        let mut too_many_checks = proposed();
        too_many_checks["unverifiedChecks"] = json!((0..5)
            .map(|index| format!("Review verification point {index}."))
            .collect::<Vec<_>>());
        assert!(parse(&too_many_checks).is_err());

        let mut duplicate_checks = proposed();
        duplicate_checks["unverifiedChecks"] = json!([
            "Review existing CI coverage.",
            "Review existing CI coverage."
        ]);
        assert!(parse(&duplicate_checks).is_err());

        let mut too_long_check = proposed();
        too_long_check["unverifiedChecks"] = json!(["界".repeat(201)]);
        assert!(parse(&too_long_check).is_err());

        let huge_raw = " ".repeat(65_537);
        assert!(parse_assessment_response(&huge_raw, &facts(), &pages()).is_err());

        let mut unicode = proposed();
        unicode["summary"] = json!("界".repeat(1_000));
        assert!(parse(&unicode).is_ok());
    }

    #[test]
    fn enforces_title_proposal_summary_reason_and_reference_count_limits() {
        let mut title_over = proposed();
        title_over["proposal"]["title"] = json!("界".repeat(241));
        assert!(parse(&title_over).is_err());

        let mut proposal_summary_over = proposed();
        proposal_summary_over["proposal"]["summary"] = json!("界".repeat(1_001));
        assert!(parse(&proposal_summary_over).is_err());

        let mut reason_over = no_proposal();
        reason_over["noProposalReason"] = json!("界".repeat(1_001));
        assert!(parse(&reason_over).is_err());

        let mut too_many_hashes = proposed();
        too_many_hashes["repositoryFactHashes"] = json!((0..33)
            .map(|index| format!("{index:064x}"))
            .collect::<Vec<_>>());
        assert!(parse(&too_many_hashes).is_err());

        let mut too_many_pages = proposed();
        too_many_pages["knowledgePageIds"] = json!((0..9)
            .map(|index| format!("workflow/page-{index}"))
            .collect::<Vec<_>>());
        assert!(parse(&too_many_pages).is_err());
    }

    #[test]
    fn accepts_unicode_at_title_and_reason_scalar_boundaries() {
        let mut title_at_limit = proposed();
        title_at_limit["proposal"]["title"] = json!("界".repeat(240));
        assert!(parse(&title_at_limit).is_ok());

        let mut reason_at_limit = no_proposal();
        reason_at_limit["noProposalReason"] = json!("界".repeat(1_000));
        assert!(parse(&reason_at_limit).is_ok());
    }

    #[test]
    fn rejects_unsupported_versions_and_target_surfaces() {
        let mut version = proposed();
        version["version"] = json!(2);
        assert!(parse(&version).is_err());

        let mut target = proposed();
        target["proposal"]["targetSurface"] = json!("shell");
        assert!(parse(&target).is_err());
    }

    #[test]
    fn retained_prose_corpus_covers_complete_workflow_responses() {
        let cases: Vec<Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/assessment-prose.json"))
                .unwrap();
        for case in cases {
            let text = &case["text"];
            for field in ["summary", "title", "proposal", "check", "reason"] {
                let expected = case["accept"].as_bool().unwrap()
                    && (case["prospectiveOnly"] != true || field == "check");
                let mut response = if field == "reason" {
                    no_proposal()
                } else {
                    proposed()
                };
                match field {
                    "summary" => response["summary"] = text.clone(),
                    "title" => response["proposal"]["title"] = text.clone(),
                    "proposal" => response["proposal"]["summary"] = text.clone(),
                    "check" => response["unverifiedChecks"] = json!([text]),
                    "reason" => response["noProposalReason"] = text.clone(),
                    _ => unreachable!(),
                }
                assert_eq!(
                    parse(&response).is_ok(),
                    expected,
                    "workflow {} in {field}: {text}",
                    case["id"]
                );
            }
        }
    }

    #[test]
    fn rejects_commands_control_characters_and_unperformed_success_claims() {
        for text in [
            "Run cargo test before merging.",
            "find . -name '*.md'",
            "python script.py",
            "ls",
            "echo",
            "python",
            "echo hello",
            "mkdir docs",
            "cd src",
            "rm output",
            "pytest tests",
            "git config user.email x",
            "All tests passed successfully.",
            "Verification completed successfully.",
            "printf hello; rm -rf /tmp/output",
            "Run go test ./...",
            "Run terraform plan",
            "Run terraform refresh",
            "Run unknowncli frobnicate",
            "Please run unknowncli frobnicate",
            "Run the unknowncli frobnicate",
            "You should execute unknowncli frobnicate",
            "Run the documentation check after implementation.",
            "Run kubectl apply -f deployment.yaml",
            "Use kubectl apply -f deployment.yaml",
            "Please call go test ./...",
            "You should launch terraform refresh",
            "Please use unknowncli frobnicate",
            "Use widget frobnicate",
            "Use the widget frobnicate",
            "Try widget frobnicate",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(parse(&value).is_err(), "unsafe text should fail: {text}");
        }

        let mut control = proposed();
        control["summary"] = json!("line\nbreak");
        assert!(parse(&control).is_err());

        let mut description = proposed();
        description["unverifiedChecks"] =
            json!(["Existing CI configuration includes regression checks."]);
        assert!(parse(&description).is_ok());
    }

    #[test]
    fn platform_descriptions_with_semicolons_are_not_commands() {
        for text in [
            "README documents setup and package metadata pins Node and pnpm versions; no supplied outcome establishes a reliable clean-checkout environment.",
            "Required versions are listed and CI specifies Node setup; local environment provisioning is unverified.",
            "The package manager and Node version are pinned; reliability remains unknown.",
            "README lists Node and pnpm requirements; availability is unknown.",
            "README and package metadata specify Node and pnpm versions, while CI selects Node 22; no evidence establishes guaranteed agent-environment availability.",
        ] {
            assert!(validate_prose(text, 500, true).is_ok(), "description rejected: {text}");
        }
        for text in [
            "Node 22; rm output",
            "Node setup; pnpm test",
            "Node package management; node script.js",
            "Node 22; printf hello",
            "node setup; local environment unknown",
        ] {
            assert!(
                validate_prose(text, 500, true).is_err(),
                "command accepted: {text}"
            );
        }
    }

    #[test]
    fn accepts_grammatical_descriptions_and_unverified_conditions() {
        for text in [
            "Make the package test script run the existing arithmetic unit test",
            "The CI configuration should make the existing test suite meaningful.",
            "The CI unit step just reports a skipped suite.",
            "The supplied facts show CI configured to run the package test suite.",
            "The package's CI script is configured to run the existing unit suite.",
            "The package script is designed to execute the existing unit suite.",
            "The client test script reports a skipped suite, while CI is configured to use that script; related test coverage remains unknown.",
            "The CI unit job is configured to use that script, consistent with the repository requirement.",
            "Whether the existing arithmetic test can run with the available package configuration.",
            "Whether the current package setup can execute the existing TypeScript test directly.",
            "The repository requires meaningful unit tests and rejects passing no-ops.",
            "This suggests CI may pass without exercising arithmetic behavior.",
            "Whether the current test file runs successfully through the proposed setup remains unverified.",
            "Whether the package test script and CI succeed after a focused setup change.",
            "Validate that the test passes and that a deliberately failing assertion is reported as a failure; neither check has been performed.",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(parse(&value).is_ok(), "description rejected: {text}");
        }
    }

    #[test]
    fn accepts_descriptive_node_prose_with_semicolons() {
        for text in [
            "Node package management is required; desktop guidance differs.",
            "The Node package configuration differs from the stated workflow. Current guidance needs review; its intended scope remains unverified.",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(parse(&value).is_ok(), "description rejected: {text}");
        }
    }

    #[test]
    fn rejects_shell_commands_separated_by_semicolons() {
        for text in [
            "cat fixture; remaining evidence is unknown.",
            "Cat fixture; remaining evidence is unknown.",
            "Bash script; remaining evidence is unknown.",
            "Node script; remaining evidence is unknown.",
            "Node-package-management; remaining evidence is unknown.",
            "Node.package.configuration; remaining evidence is unknown.",
            "The validation scope includes cat fixture; remaining evidence is unknown.",
            "Node tools/check.js; remaining evidence is unknown.",
            "Node --version; remaining evidence is unknown.",
            "The validation scope includes node tools/check.js; remaining evidence is unknown.",
            "The guidance mentions Node; rm output.",
            "The fallback is printf hello; remaining evidence is unknown.",
            "The fallback is bash script; remaining evidence is unknown.",
            "The fallback is ls src; remaining evidence is unknown.",
            "The scope includes Node --version; remaining evidence is unknown.",
            "The guidance mentions Node; `unknowncli frobnicate`.",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(parse(&value).is_err(), "command accepted: {text}");
        }
    }

    #[test]
    fn rejects_commands_and_claims_inside_otherwise_qualified_prose() {
        for text in [
            "Whether checks succeed is unknown. All tests passed.",
            "Tests have not been run; all tests passed.",
            "No tests failed. All tests passed.",
            "Tests have not been run, but all tests passed.",
            "Whether tests pass is unknown; run unknowncli frobnicate.",
            "The setup is configured to run cargo test.",
            "The CI script is configured to run rm -rf /.",
            "The CI job is configured to use that script; rm output.",
            "The CI job is configured to use that script to run unknowncli frobnicate.",
            "The CI job is configured to use that script rm output.",
            "The package script is designed to execute python tools/check.py.",
            "Whether CI can run curl https://example.test remains unverified.",
            "The workflow is configured to invoke bash -c anything.",
            "The CI script is configured to run unknowncli frobnicate.",
            "The CI script is configured to run the existing command rm -rf /.",
            "The CI script is configured to execute test -e /tmp/output.",
            "The CI script is configured to run the existing command unknowncli frobnicate.",
            "The CI script is configured to execute the existing test -e /tmp/output.",
            "The CI script is configured to run the existing `unknowncli frobnicate`.",
            "The CI script is configured to execute the current \"unknowncli frobnicate\".",
            "The CI script is configured to execute the current 'unknowncli frobnicate'.",
            "The CI script is configured to run the existing (`unknowncli frobnicate`).",
            "The CI script is configured to execute the current (\"unknowncli frobnicate\").",
            "The CI script is configured to execute the current ('unknowncli frobnicate').",
            "Whether CI can run cargo test is unknown.",
            "You can run unknowncli frobnicate.",
            "You need to execute unknowncli frobnicate.",
            "You are expected to run unknowncli frobnicate.",
            "The reader is configured to execute unknowncli frobnicate.",
            "We can confirm tests passed.",
            "Tests can be improved because all checks passed.",
            "Make test",
            "Just deploy",
            "The proposed check is `make the`.",
            "The proposed check is `just deploy`.",
            "Tests passed without regressions.",
            "The CI unit tests passed and may need more coverage.",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(parse(&value).is_err(), "unsafe text accepted: {text}");
        }
    }

    #[test]
    fn accepts_descriptions_that_mention_tools_runs_and_unverified_checks() {
        for text in [
            "Find ways to document the workflow.",
            "Python workflows need documentation.",
            "Document the Git workflow.",
            "Compare run evidence.",
            "Explain how tests should pass.",
            "Run through the plan.",
            "Use repository evidence to prioritize the next improvement.",
            "Tests have not been verified.",
            "Check whether tests pass.",
        ] {
            let mut value = proposed();
            value["summary"] = json!(text);
            assert!(
                parse(&value).is_ok(),
                "ordinary description should parse: {text}"
            );
        }
    }
}
