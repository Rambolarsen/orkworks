//! Pure supplied-instruction projection, not a configuration codec or launch grant.
#![allow(dead_code)] // Dormant until a separately approved configuration consumer exists.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstructionRole {
    Orchestrator,
    Research,
    Implementation,
    Review,
    Verification,
    Remediation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SkillRequirement {
    Mandatory,
    Optional,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoleInstruction {
    pub id: String,
    pub role: InstructionRole,
    pub instructions: String,
    pub instructions_digest: String,
    pub source_snapshot_digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleInstruction {
    pub id: String,
    pub source_identity: String,
    pub source_path: String,
    pub precedence: usize,
    pub authority: String,
    pub content: String,
    pub content_digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SkillInstruction {
    pub id: String,
    pub content: String,
    pub content_digest: String,
    pub requirement: SkillRequirement,
    pub requirement_source_ids: Vec<String>,
    pub resource_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResourceInstruction {
    pub id: String,
    pub skill_id: String,
    pub content: String,
    pub content_digest: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstructionRequirement {
    pub source_id: String,
    pub source_digest: String,
    pub task_id: String,
    pub required_skill_ids: Vec<String>,
    pub source_locations: Vec<String>,
    pub applicability_reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstructionConflict {
    pub source_ids: [String; 2],
    pub explanation: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstructionBundleInput {
    pub task_id: String,
    pub coordination_text: String,
    pub assignment_text: String,
    pub role: RoleInstruction,
    pub rules: Vec<RuleInstruction>,
    pub skills: Vec<SkillInstruction>,
    pub resources: Vec<ResourceInstruction>,
    pub requirements: Vec<InstructionRequirement>,
    pub reported_unresolved_source_ids: Vec<String>,
    pub reported_conflicts: Vec<InstructionConflict>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct InstructionBundleError {
    pub invariant: &'static str,
    pub index: Option<usize>,
}
#[derive(Debug)]
pub(crate) struct RenderedInstructionBundle {
    text: String,
    sha256: String,
}
impl RenderedInstructionBundle {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
}
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_RENDERED: usize = 256 * 1024;
fn invalid(invariant: &'static str, index: Option<usize>) -> InstructionBundleError {
    InstructionBundleError { invariant, index }
}
fn check(
    condition: bool,
    invariant: &'static str,
    index: Option<usize>,
) -> Result<(), InstructionBundleError> {
    if condition {
        Ok(())
    } else {
        Err(invalid(invariant, index))
    }
}
fn id(value: &str) -> Result<(), InstructionBundleError> {
    check(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)),
        "id",
        None,
    )
}
fn digest(value: &str) -> Result<(), InstructionBundleError> {
    check(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
        None,
    )
}
fn text_digest(content: &str, expected: &str) -> Result<(), InstructionBundleError> {
    check(content.len() <= MAX_RENDERED, "content bytes", None)?;
    digest(expected)?;
    check(sha256(content) == expected, "content digest", None)
}
fn sha256(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}
fn id_set<'a>(
    values: &'a [String],
    max: usize,
    field: &'static str,
    index: usize,
) -> Result<BTreeSet<&'a str>, InstructionBundleError> {
    check(values.len() <= max, field, Some(index))?;
    let mut set = BTreeSet::new();
    for value in values {
        id(value)?;
        check(set.insert(value.as_str()), field, Some(index))?;
    }
    Ok(set)
}
fn checked_total(
    total: usize,
    extra: usize,
    limit: usize,
) -> Result<usize, InstructionBundleError> {
    total
        .checked_add(extra)
        .filter(|n| *n <= limit)
        .ok_or_else(|| invalid("byte limit", None))
}
fn validate_blockers(input: &InstructionBundleInput) -> Result<(), InstructionBundleError> {
    let mut bytes = 0;
    let refs = input
        .reported_conflicts
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(input.reported_unresolved_source_ids.len()));
    check(refs.is_some_and(|n| n <= 16), "blocker references", None)?;
    for source in &input.reported_unresolved_source_ids {
        id(source)?;
        bytes = checked_total(bytes, source.len(), 16 * 1024)?;
    }
    for (index, conflict) in input.reported_conflicts.iter().enumerate() {
        for source in &conflict.source_ids {
            id(source)?;
            bytes = checked_total(bytes, source.len(), 16 * 1024)?;
        }
        check(
            !conflict.explanation.trim().is_empty() && conflict.explanation.len() <= 2048,
            "conflict reason",
            Some(index),
        )?;
        bytes = checked_total(bytes, conflict.explanation.len(), 16 * 1024)?;
    }
    check(
        input.reported_unresolved_source_ids.is_empty(),
        "reported unresolved source",
        Some(0),
    )?;
    check(
        input.reported_conflicts.is_empty(),
        "reported conflict",
        Some(0),
    )
}
fn validate(input: &InstructionBundleInput) -> Result<(), InstructionBundleError> {
    validate_blockers(input)?;
    id(&input.task_id)?;
    id(&input.role.id)?;
    check(
        input.coordination_text.len() <= MAX_RENDERED
            && input.assignment_text.len() <= MAX_RENDERED,
        "content bytes",
        None,
    )?;
    check(
        input.skills.len() <= 16
            && input
                .rules
                .len()
                .checked_add(input.resources.len())
                .is_some_and(|n| n <= 32)
            && input.requirements.len() <= 33,
        "collection limit",
        None,
    )?;
    text_digest(&input.role.instructions, &input.role.instructions_digest)?;
    digest(&input.role.source_snapshot_digest)?;
    let mut sources = BTreeMap::from([(
        input.role.id.as_str(),
        input.role.source_snapshot_digest.as_str(),
    )]);
    let mut locators = BTreeSet::new();
    for (index, rule) in input.rules.iter().enumerate() {
        id(&rule.id)?;
        id(&rule.source_identity)?;
        check(
            sources
                .insert(rule.id.as_str(), rule.content_digest.as_str())
                .is_none(),
            "source id",
            Some(index),
        )?;
        check(rule.precedence == index, "rule precedence", Some(index))?;
        check(rule.authority == "binding", "rule authority", Some(index))?;
        let path = &rule.source_path;
        check(
            !path.is_empty()
                && path.len() <= 2048
                && !path.starts_with('/')
                && path.as_bytes().get(1) != Some(&b':')
                && !path.contains(['\\', '\0'])
                && path
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != ".."),
            "rule path",
            Some(index),
        )?;
        check(
            locators.insert((rule.source_identity.as_str(), path.as_str())),
            "rule locator",
            Some(index),
        )?;
        text_digest(&rule.content, &rule.content_digest)?;
    }
    let mut skills = BTreeMap::new();
    for (index, skill) in input.skills.iter().enumerate() {
        id(&skill.id)?;
        text_digest(&skill.content, &skill.content_digest)?;
        check(
            skills.insert(skill.id.as_str(), skill).is_none(),
            "skill id",
            Some(index),
        )?;
        id_set(
            &skill.requirement_source_ids,
            32,
            "requirement sources",
            index,
        )?;
        id_set(&skill.resource_ids, 32, "resource ids", index)?;
    }
    let mut resources = BTreeMap::new();
    for (index, resource) in input.resources.iter().enumerate() {
        id(&resource.id)?;
        id(&resource.skill_id)?;
        text_digest(&resource.content, &resource.content_digest)?;
        check(
            resources.insert(resource.id.as_str(), resource).is_none(),
            "resource id",
            Some(index),
        )?;
    }
    let mut manifest_sources = BTreeSet::new();
    let mut contributors: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (index, requirement) in input.requirements.iter().enumerate() {
        id(&requirement.source_id)?;
        digest(&requirement.source_digest)?;
        id(&requirement.task_id)?;
        check(
            sources.contains_key(requirement.source_id.as_str())
                && manifest_sources.insert(requirement.source_id.as_str()),
            "manifest source",
            Some(index),
        )?;
        check(
            sources[requirement.source_id.as_str()] == requirement.source_digest
                && requirement.task_id == input.task_id,
            "manifest binding",
            Some(index),
        )?;
        let required = id_set(
            &requirement.required_skill_ids,
            16,
            "required skill ids",
            index,
        )?;
        check(
            requirement.applicability_reason.len() <= 2048
                && (!required.is_empty() || !requirement.applicability_reason.trim().is_empty()),
            "applicability reason",
            Some(index),
        )?;
        check(
            requirement.source_locations.len() <= 32,
            "source locations",
            Some(index),
        )?;
        let mut locations = BTreeSet::new();
        for location in &requirement.source_locations {
            check(
                !location.is_empty()
                    && location.len() <= 512
                    && locations.insert(location.as_str()),
                "source locations",
                Some(index),
            )?;
        }
        for skill_id in required {
            check(
                skills
                    .get(skill_id)
                    .is_some_and(|skill| skill.requirement == SkillRequirement::Mandatory),
                "required skill",
                Some(index),
            )?;
            contributors
                .entry(skill_id)
                .or_default()
                .insert(requirement.source_id.as_str());
        }
    }
    check(
        manifest_sources.len() == sources.len(),
        "manifest sources",
        None,
    )?;
    for (index, skill) in input.skills.iter().enumerate() {
        let provided = id_set(
            &skill.requirement_source_ids,
            32,
            "requirement sources",
            index,
        )?;
        let expected = contributors
            .get(skill.id.as_str())
            .cloned()
            .unwrap_or_default();
        check(
            provided == expected
                && (skill.requirement == SkillRequirement::Mandatory) == !expected.is_empty(),
            "requirement sources",
            Some(index),
        )?;
        for resource_id in &skill.resource_ids {
            check(
                resources
                    .get(resource_id.as_str())
                    .is_some_and(|resource| resource.skill_id == skill.id),
                "resource closure",
                Some(index),
            )?;
        }
    }
    for (index, resource) in input.resources.iter().enumerate() {
        check(
            skills
                .get(resource.skill_id.as_str())
                .is_some_and(|skill| skill.resource_ids.contains(&resource.id)),
            "resource closure",
            Some(index),
        )?;
    }
    Ok(())
}

/// Checks only supplied instruction structure/bytes; never grants configuration or launch authority.
pub(crate) fn render_instruction_bundle(
    input: &InstructionBundleInput,
) -> Result<RenderedInstructionBundle, InstructionBundleError> {
    validate(input)?;
    // Rendering is the next independently tested task.
    Ok(RenderedInstructionBundle {
        text: String::new(),
        sha256: String::new(),
    })
}
