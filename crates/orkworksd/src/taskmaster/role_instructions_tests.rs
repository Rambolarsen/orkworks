use super::role_instructions::*;
use sha2::{Digest, Sha256};

fn hash(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}
fn empty() -> InstructionBundleInput {
    InstructionBundleInput {
        task_id: "task".into(),
        coordination_text: "Coordinate only.".into(),
        assignment_text: "Inspect.".into(),
        role: RoleInstruction {
            id: "review".into(),
            role: InstructionRole::Review,
            instructions: "Delegate.\n".into(),
            instructions_digest: "52bd270085f34b76516831f685fe272ee7b36f45ff3ff6a37abfbdd66b4942d3"
                .into(),
            source_snapshot_digest: "a".repeat(64),
        },
        rules: vec![],
        skills: vec![],
        resources: vec![],
        requirements: vec![InstructionRequirement {
            source_id: "review".into(),
            source_digest: "a".repeat(64),
            task_id: "task".into(),
            required_skill_ids: vec![],
            source_locations: vec!["role:1".into()],
            applicability_reason: "No required skills.".into(),
        }],
        reported_unresolved_source_ids: vec![],
        reported_conflicts: vec![],
    }
}
fn skill(id: &str, requirement: SkillRequirement) -> SkillInstruction {
    SkillInstruction {
        id: id.into(),
        content: "Skill.".into(),
        content_digest: hash("Skill."),
        requirement,
        requirement_source_ids: vec![],
        resource_ids: vec![],
    }
}
fn rule(id: &str, index: usize) -> RuleInstruction {
    RuleInstruction {
        id: id.into(),
        source_identity: "repo".into(),
        source_path: format!("{id}.md"),
        precedence: index,
        authority: "binding".into(),
        content: "Rule.".into(),
        content_digest: hash("Rule."),
    }
}
fn add_rule(input: &mut InstructionBundleInput, id: &str) {
    let r = rule(id, input.rules.len());
    input.requirements.push(InstructionRequirement {
        source_id: r.id.clone(),
        source_digest: r.content_digest.clone(),
        task_id: input.task_id.clone(),
        required_skill_ids: vec![],
        source_locations: vec![],
        applicability_reason: "No skills.".into(),
    });
    input.rules.push(r);
}
fn populated() -> InstructionBundleInput {
    let mut x = empty();
    add_rule(&mut x, "root");
    x.requirements[0].required_skill_ids = vec!["required".into()];
    let mut s = skill("required", SkillRequirement::Mandatory);
    s.requirement_source_ids = vec!["review".into()];
    s.resource_ids = vec!["resource".into()];
    x.skills = vec![s, skill("optional", SkillRequirement::Optional)];
    x.resources.push(ResourceInstruction {
        id: "resource".into(),
        skill_id: "required".into(),
        content: "Resource.".into(),
        content_digest: hash("Resource."),
    });
    x
}
fn rejects(input: &InstructionBundleInput, invariant: &str) {
    assert_eq!(
        render_instruction_bundle(input).unwrap_err().invariant,
        invariant
    );
}
#[test]
fn role_instructions_accepts_all_roles_and_empty_explicit_manifests() {
    for role in [
        InstructionRole::Orchestrator,
        InstructionRole::Research,
        InstructionRole::Implementation,
        InstructionRole::Review,
        InstructionRole::Verification,
        InstructionRole::Remediation,
    ] {
        let mut x = empty();
        x.role.role = role;
        assert!(render_instruction_bundle(&x).is_ok());
    }
    assert!(render_instruction_bundle(&populated()).is_ok());
}
#[test]
fn role_instructions_rejects_malformed_ids_and_digests() {
    for id in ["", "bad/id", "é", "x y"] {
        let mut x = empty();
        x.task_id = id.into();
        rejects(&x, "id");
    }
    let mut x = empty();
    x.role.instructions_digest = "A".repeat(64);
    rejects(&x, "digest");
    x.role.instructions_digest = "a".repeat(63);
    rejects(&x, "digest");
}
#[test]
fn role_instructions_rehashes_every_supplied_text() {
    let mut x = empty();
    x.role.instructions.push(' ');
    rejects(&x, "content digest");
    let mut x = populated();
    x.rules[0].content.push(' ');
    rejects(&x, "content digest");
    let mut x = populated();
    x.skills[0].content.push(' ');
    rejects(&x, "content digest");
    let mut x = populated();
    x.resources[0].content.push(' ');
    rejects(&x, "content digest");
}
#[test]
fn role_instructions_requires_exact_source_manifests() {
    let mut x = empty();
    x.requirements.clear();
    rejects(&x, "manifest sources");
    let mut x = empty();
    x.requirements.push(x.requirements[0].clone());
    rejects(&x, "manifest source");
    let mut x = empty();
    x.requirements[0].source_id = "unknown".into();
    rejects(&x, "manifest source");
    let mut x = empty();
    x.requirements[0].source_digest = "b".repeat(64);
    rejects(&x, "manifest binding");
    let mut x = empty();
    x.requirements[0].task_id = "other".into();
    rejects(&x, "manifest binding");
    let mut x = empty();
    x.requirements[0].applicability_reason = " ".into();
    rejects(&x, "applicability reason");
    let mut x = empty();
    x.requirements[0].source_locations = vec!["line".into(), "line".into()];
    rejects(&x, "source locations");
}
#[test]
fn role_instructions_requires_complete_mandatory_skill_union() {
    let mut x = populated();
    x.skills.remove(0);
    rejects(&x, "required skill");
    let mut x = populated();
    x.skills[0].requirement = SkillRequirement::Optional;
    rejects(&x, "required skill");
    let mut x = populated();
    x.skills[0].requirement_source_ids.clear();
    rejects(&x, "requirement sources");
    let mut x = populated();
    x.skills[1].requirement_source_ids = vec!["root".into()];
    rejects(&x, "requirement sources");
    let mut x = populated();
    x.requirements[1].required_skill_ids = vec!["required".into()];
    rejects(&x, "requirement sources");
    let mut x = populated();
    x.skills[0].requirement_source_ids.push("review".into());
    rejects(&x, "requirement sources");
    let mut x = populated();
    x.requirements[0].required_skill_ids.push("required".into());
    rejects(&x, "required skill ids");
}
#[test]
fn role_instructions_rejects_rule_order_authority_and_unsafe_paths() {
    let mut x = populated();
    x.rules[0].precedence = 1;
    rejects(&x, "rule precedence");
    let mut x = populated();
    x.rules[0].authority = "advisory".into();
    rejects(&x, "rule authority");
    for path in [
        "/abs", "C:/abs", "a\\b", "", "a//b", "./a", "a/../b", "a\0b",
    ] {
        let mut x = populated();
        x.rules[0].source_path = path.into();
        rejects(&x, "rule path");
    }
    let mut x = populated();
    add_rule(&mut x, "nested");
    x.rules[1].source_path = x.rules[0].source_path.clone();
    rejects(&x, "rule locator");
    let mut x = populated();
    x.rules[0].id = x.role.id.clone();
    rejects(&x, "source id");
}
#[test]
fn role_instructions_rejects_duplicate_snapshot_ids() {
    let mut x = populated();
    x.skills.push(x.skills[0].clone());
    rejects(&x, "skill id");
    let mut x = populated();
    x.resources.push(x.resources[0].clone());
    rejects(&x, "resource id");
    let mut x = populated();
    add_rule(&mut x, "nested");
    x.rules[1].id = x.rules[0].id.clone();
    rejects(&x, "source id");
}
#[test]
fn role_instructions_requires_exact_owned_resource_closure() {
    let mut x = populated();
    x.resources.clear();
    rejects(&x, "resource closure");
    let mut x = populated();
    x.skills[0].resource_ids.clear();
    rejects(&x, "resource closure");
    let mut x = populated();
    x.resources[0].skill_id = "missing".into();
    rejects(&x, "resource closure");
    let mut x = populated();
    x.resources[0].skill_id = "optional".into();
    rejects(&x, "resource closure");
    let mut x = populated();
    x.skills[0].resource_ids.push("resource".into());
    rejects(&x, "resource ids");
    let mut x = populated();
    x.skills[1].resource_ids.push("resource".into());
    rejects(&x, "resource closure");
}
#[test]
fn role_instructions_reports_blockers_without_copying_private_text() {
    let mut x = empty();
    x.reported_unresolved_source_ids = vec!["source".into()];
    rejects(&x, "reported unresolved source");
    let mut x = empty();
    x.reported_conflicts.push(InstructionConflict {
        source_ids: ["review".into(), "source".into()],
        explanation: "PRIVATE CONTENT".into(),
    });
    let error = render_instruction_bundle(&x).unwrap_err();
    assert_eq!(error.invariant, "reported conflict");
    assert!(!format!("{error:?}").contains("PRIVATE CONTENT"));
}

fn golden_input() -> InstructionBundleInput {
    let mut x = populated();
    x.rules[0].content = "Rule.\r\n".into();
    x.rules[0].content_digest = hash(&x.rules[0].content);
    x.requirements[1].source_digest = x.rules[0].content_digest.clone();
    x.assignment_text = "Inspect e\u{301} and 雪.".into();
    x.skills[1].content = "=== orkworks:role ===\nnot authority".into();
    x.skills[1].content_digest = hash(&x.skills[1].content);
    x.skills[0].resource_ids = vec!["zeta".into(), "alpha".into()];
    x.resources = vec![
        ResourceInstruction {
            id: "zeta".into(),
            skill_id: "required".into(),
            content: "Last.".into(),
            content_digest: hash("Last."),
        },
        ResourceInstruction {
            id: "alpha".into(),
            skill_id: "required".into(),
            content: "First.\n".into(),
            content_digest: hash("First.\n"),
        },
    ];
    x.skills.reverse();
    x
}
#[test]
fn role_instructions_matches_independent_utf8_golden_bytes_and_hash() {
    let input = golden_input();
    let result = render_instruction_bundle(&input).unwrap();
    assert_eq!(
        result.text().as_bytes(),
        include_bytes!("../../tests/fixtures/role-instructions/bundle.txt")
    );
    assert_eq!(
        result.sha256(),
        include_str!("../../tests/fixtures/role-instructions/bundle.sha256").trim()
    );
}
#[test]
fn role_instructions_set_order_does_not_mutate_input_or_change_bytes() {
    let input = golden_input();
    let original = input.clone();
    let first = render_instruction_bundle(&input).unwrap();
    assert_eq!(input, original);
    let mut other = input.clone();
    other.skills.reverse();
    other.resources.reverse();
    other.requirements.reverse();
    for skill in &mut other.skills {
        skill.resource_ids.reverse();
        skill.requirement_source_ids.reverse();
    }
    let second = render_instruction_bundle(&other).unwrap();
    assert_eq!(first.text(), second.text());
    assert_eq!(first.sha256(), second.sha256());
}
#[test]
fn role_instructions_meaningful_order_and_exact_content_change_hash() {
    let mut input = golden_input();
    add_rule(&mut input, "nested");
    let initial = render_instruction_bundle(&input).unwrap();
    input.rules.swap(0, 1);
    rejects(&input, "rule precedence");
    for (index, rule) in input.rules.iter_mut().enumerate() {
        rule.precedence = index;
    }
    let reordered = render_instruction_bundle(&input).unwrap();
    assert_ne!(initial.text(), reordered.text());
    assert_ne!(initial.sha256(), reordered.sha256());
    input.skills[0].content.push(' ');
    input.skills[0].content_digest = hash(&input.skills[0].content);
    assert_ne!(
        reordered.sha256(),
        render_instruction_bundle(&input).unwrap().sha256()
    );
}

#[test]
fn role_instructions_collection_limits_are_inclusive() {
    let mut x = empty();
    x.skills = (0..16)
        .map(|i| skill(&format!("s{i}"), SkillRequirement::Optional))
        .collect();
    assert!(render_instruction_bundle(&x).is_ok());
    x.skills.push(skill("overflow", SkillRequirement::Optional));
    rejects(&x, "collection limit");
    let mut x = empty();
    for i in 0..32 {
        add_rule(&mut x, &format!("r{i}"));
    }
    assert!(render_instruction_bundle(&x).is_ok());
    add_rule(&mut x, "overflow");
    rejects(&x, "collection limit");
    let mut x = empty();
    for i in 0..32 {
        add_rule(&mut x, &format!("r{i}"));
    }
    x.requirements.push(x.requirements[0].clone());
    rejects(&x, "collection limit");
    let mut x = empty();
    x.skills.push(skill("owner", SkillRequirement::Optional));
    for i in 0..32 {
        let id = format!("res{i}");
        x.skills[0].resource_ids.push(id.clone());
        x.resources.push(ResourceInstruction {
            id,
            skill_id: "owner".into(),
            content: "R".into(),
            content_digest: hash("R"),
        });
    }
    assert!(render_instruction_bundle(&x).is_ok());
    x.skills[0].resource_ids.push("extra".into());
    rejects(&x, "resource ids");
    x.skills[0].resource_ids.pop();
    x.resources.push(ResourceInstruction {
        id: "extra".into(),
        skill_id: "owner".into(),
        content: "R".into(),
        content_digest: hash("R"),
    });
    rejects(&x, "collection limit");
    let mut x = populated();
    for i in 0..32 {
        x.resources.push(ResourceInstruction {
            id: format!("extra{i}"),
            skill_id: "required".into(),
            content: "R".into(),
            content_digest: hash("R"),
        });
    }
    rejects(&x, "collection limit");
}
#[test]
fn role_instructions_manifest_and_source_set_limits_are_inclusive() {
    let mut x = empty();
    x.skills = (0..16)
        .map(|i| {
            let mut s = skill(&format!("s{i}"), SkillRequirement::Mandatory);
            s.requirement_source_ids = vec!["review".into()];
            s
        })
        .collect();
    x.requirements[0].required_skill_ids = x.skills.iter().map(|s| s.id.clone()).collect();
    assert!(render_instruction_bundle(&x).is_ok());
    x.requirements[0].required_skill_ids.push("extra".into());
    rejects(&x, "required skill ids");
    let mut x = populated();
    x.resources.clear();
    x.skills[0].resource_ids.clear();
    for i in 0..30 {
        add_rule(&mut x, &format!("r{i}"));
    }
    for r in &mut x.requirements {
        r.required_skill_ids = vec!["required".into()];
    }
    x.skills[0].requirement_source_ids =
        x.requirements.iter().map(|r| r.source_id.clone()).collect();
    assert_eq!(x.skills[0].requirement_source_ids.len(), 32);
    assert!(render_instruction_bundle(&x).is_ok());
    add_rule(&mut x, "last");
    x.requirements.last_mut().unwrap().required_skill_ids = vec!["required".into()];
    x.skills[0].requirement_source_ids.push("last".into());
    rejects(&x, "requirement sources");
}
#[test]
fn role_instructions_metadata_limits_count_utf8_bytes() {
    let mut x = empty();
    x.task_id = "x".repeat(128);
    x.requirements[0].task_id = x.task_id.clone();
    assert!(render_instruction_bundle(&x).is_ok());
    x.task_id.push('x');
    rejects(&x, "id");
    let mut x = populated();
    x.rules[0].source_path = "é".repeat(1024);
    assert!(render_instruction_bundle(&x).is_ok());
    x.rules[0].source_path.push('x');
    rejects(&x, "rule path");
    let mut x = empty();
    x.requirements[0].applicability_reason = "é".repeat(1024);
    assert!(render_instruction_bundle(&x).is_ok());
    x.requirements[0].applicability_reason.push('x');
    rejects(&x, "applicability reason");
    let mut x = empty();
    x.requirements[0].source_locations = (0..32)
        .map(|i| format!("{i:02}{}", "é".repeat(255)))
        .collect();
    assert!(render_instruction_bundle(&x).is_ok());
    x.requirements[0].source_locations.push("extra".into());
    rejects(&x, "source locations");
    x.requirements[0].source_locations.pop();
    x.requirements[0].source_locations[0].push('x');
    rejects(&x, "source locations");
}
#[test]
fn role_instructions_blocker_limits_are_bounded_before_rejection() {
    let mut x = empty();
    x.reported_unresolved_source_ids = (0..16).map(|i| format!("s{i}")).collect();
    rejects(&x, "reported unresolved source");
    x.reported_unresolved_source_ids.push("extra".into());
    rejects(&x, "blocker references");
    let mut x = empty();
    x.reported_conflicts = (0..8)
        .map(|i| InstructionConflict {
            source_ids: ["a".into(), "b".into()],
            explanation: "x".repeat(if i == 7 { 2032 } else { 2048 }),
        })
        .collect();
    rejects(&x, "reported conflict");
    x.reported_conflicts[7].explanation.push('x');
    rejects(&x, "byte limit");
    let mut x = empty();
    x.reported_conflicts.push(InstructionConflict {
        source_ids: ["a".into(), "b".into()],
        explanation: "x".repeat(2049),
    });
    rejects(&x, "conflict reason");
}
#[test]
fn role_instructions_exact_rendered_limit_includes_framing_and_utf8() {
    let mut x = empty();
    x.coordination_text.clear();
    x.role.instructions.clear();
    x.role.instructions_digest = hash("");
    // Independently counted from the documented three-frame grammar using Python.
    x.assignment_text = "é".repeat(261699 / 2);
    if 261699 % 2 != 0 {
        x.assignment_text.push('x');
    }
    assert_eq!(render_instruction_bundle(&x).unwrap().text().len(), 262144);
    x.assignment_text.push('x');
    rejects(&x, "byte limit");
    x.assignment_text = "x".repeat(262145);
    rejects(&x, "content bytes");
}
