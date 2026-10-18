//! Repository-owned evidence selection for manual assessment only.
use super::{context, runtime::ContextLevel, RepositoryEvidence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};

pub(super) const COLLECTOR_VERSION: u8 = 2;
const MAX_PREFIX: usize = 8192;
const MAX_TOTAL: usize = 65536;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InputManifest {
    pub context_level: ContextLevel,
    pub excluded_paths: Vec<String>,
    pub collector_version: u8,
    pub prompt_version: String,
    pub catalog_digest: String,
    pub catalog_complete: bool,
    pub instruction_coverage_complete: bool,
    pub files: Vec<ManifestFile>,
    pub requested_file_ids: Vec<usize>,
    pub discovery_digest: Option<String>,
    pub redacted: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ManifestFile {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
    pub seed: bool,
    pub excerpt_start: usize,
    pub excerpt_end: usize,
    pub excerpt_digest: String,
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn valid_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
impl InputManifest {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.collector_version != COLLECTOR_VERSION
            || self.prompt_version != super::assessment::ASSESSMENT_PROMPT_VERSION
            || self.files.len() > 32
            || self.requested_file_ids.len() > 8
            || self.requested_file_ids.iter().any(|id| *id >= 512)
            || self.requested_file_ids.iter().collect::<HashSet<_>>().len()
                != self.requested_file_ids.len()
        {
            return Err("Assessment input manifest is invalid".into());
        }
        if self.redacted {
            if !self.files.is_empty()
                || !self.catalog_digest.is_empty()
                || self.discovery_digest.is_some()
                || !self.requested_file_ids.is_empty()
            {
                return Err("Redacted assessment manifest retains evidence".into());
            }
            return Ok(());
        }
        if !valid_digest(&self.catalog_digest)
            || self
                .discovery_digest
                .as_ref()
                .is_some_and(|d| !valid_digest(d))
        {
            return Err("Assessment manifest digest is invalid".into());
        }
        let mut paths = HashSet::new();
        let mut total = 0;
        let mut seed_total = 0;
        let mut seed_count = 0;
        let mut followup_seen = false;
        for file in &self.files {
            if !context::fact_is_permitted(&file.path, ContextLevel::SourceCode, &[])
                || canonical_relative_path(Path::new(&file.path)).as_deref()
                    != Some(file.path.as_str())
                || file.path.len() > 512
                || file.path.chars().any(char::is_control)
                || !paths.insert(file.path.as_str())
                || !valid_digest(&file.sha256)
                || !valid_digest(&file.excerpt_digest)
                || file.bytes > MAX_PREFIX
                || file.excerpt_start > file.excerpt_end
                || file.excerpt_end > file.bytes
                || (file.seed && followup_seen)
            {
                return Err("Assessment manifest file is invalid".into());
            }
            total += file.bytes;
            if file.seed {
                seed_total += file.bytes;
                seed_count += 1;
            } else {
                followup_seen = true;
            }
        }
        if total > MAX_TOTAL || seed_total > 24576 || seed_count > 16 {
            return Err("Assessment manifest exceeds evidence limits".into());
        }
        Ok(())
    }
    pub(crate) fn redact(&mut self) {
        self.redacted = true;
        self.files.clear();
        self.catalog_digest.clear();
        self.requested_file_ids.clear();
        self.discovery_digest = None;
    }
}

#[derive(Clone)]
pub(crate) struct RepositoryContext {
    root: PathBuf,
    level: ContextLevel,
    excluded: Vec<String>,
    observed_at: String,
    pub catalog: Vec<String>,
    pub facts: Vec<RepositoryEvidence>,
    prefixes: BTreeMap<String, Vec<u8>>,
    manifest: InputManifest,
    coverage: context::TraversalCoverage,
    permitted: HashSet<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FollowUp {
    Read,
    EvidenceLimit,
}

impl RepositoryContext {
    pub(crate) fn collect(
        workspace: &Path,
        level: ContextLevel,
        excluded: &[String],
        observed_at: &str,
    ) -> Result<Self, String> {
        if level == ContextLevel::SessionObservations {
            return Err("Repository assessment requires workflow context access".into());
        }
        if workspace.to_str().is_none() {
            return Err("Assessment workspace path is not valid UTF-8".into());
        }
        let root = workspace
            .canonicalize()
            .map_err(|_| "Assessment workspace is unavailable")?;
        if root.to_str().is_none() || workspace.to_str().is_none() {
            return Err("Assessment workspace path is not valid UTF-8".into());
        }
        let repository = git2::Repository::discover(&root).ok();
        let mut paths = Vec::new();
        let mut remaining = 2000;
        let mut complete = true;
        let mut coverage = context::TraversalCoverage::default();
        context::collect_paths(
            &root,
            &root,
            0,
            level,
            excluded,
            repository.as_ref(),
            &mut paths,
            &mut remaining,
            &[],
            &mut complete,
            &mut coverage,
        )?;
        paths.sort_by_cached_key(|p| {
            context::context_priority(p.strip_prefix(&root).unwrap_or(p), &[])
        });
        let mut catalog = Vec::new();
        let mut permitted = HashSet::new();
        let mut size = 2;
        complete &= remaining > 0;
        for path in paths {
            let Some(relative) = path
                .strip_prefix(&root)
                .ok()
                .and_then(canonical_relative_path)
            else {
                complete = false;
                continue;
            };
            if relative.len() > 512 || relative.chars().any(char::is_control) {
                complete = false;
                continue;
            }
            permitted.insert(relative.clone());
            // A catalog entry is permission data, not proof of absence. Vet the
            // bounded prefix without retaining or supplying unselected contents.
            if read_prefix(&root, &relative, level, excluded).is_err() {
                continue;
            }
            let encoded = serde_json::to_string(&relative).map_err(|_| "Invalid catalog path")?;
            if catalog.len() == 512 || size + encoded.len() + 1 > 16384 {
                complete = false;
                continue;
            }
            size += encoded.len() + 1;
            catalog.push(relative);
        }
        let catalog_digest = digest(&serde_json::to_vec(&catalog).map_err(|_| "Invalid catalog")?);
        let mut result = Self {
            root,
            level,
            excluded: excluded.to_vec(),
            observed_at: observed_at.into(),
            catalog,
            facts: Vec::new(),
            prefixes: BTreeMap::new(),
            coverage,
            permitted,
            manifest: InputManifest {
                context_level: level,
                excluded_paths: excluded.to_vec(),
                collector_version: COLLECTOR_VERSION,
                prompt_version: super::assessment::ASSESSMENT_PROMPT_VERSION.into(),
                catalog_digest,
                catalog_complete: complete,
                instruction_coverage_complete: true,
                files: Vec::new(),
                requested_file_ids: Vec::new(),
                discovery_digest: None,
                redacted: false,
            },
        };
        let mut seeds = result.catalog.clone();
        seeds.sort_by_cached_key(|p| context::context_priority(Path::new(p), &[]));
        for path in seeds {
            if result.facts.len() == 16 {
                break;
            }
            if !context::fact_is_permitted(&path, ContextLevel::WorkflowContext, excluded) {
                continue;
            }
            let Ok(paths) = result.instruction_closure(std::slice::from_ref(&path)) else {
                result.manifest.instruction_coverage_complete = false;
                continue;
            };
            let mut pending = Vec::new();
            let mut denied = false;
            for path in paths {
                if result.prefixes.contains_key(&path) {
                    continue;
                }
                match read_prefix(&result.root, &path, level, excluded) {
                    Ok(bytes) => pending.push((path, bytes)),
                    Err(_) => {
                        denied = true;
                        break;
                    }
                }
            }
            let total: usize = result.prefixes.values().map(Vec::len).sum();
            if denied
                || result.facts.len() + pending.len() > 16
                || total + pending.iter().map(|(_, b)| b.len()).sum::<usize>() > 24576
            {
                result.manifest.instruction_coverage_complete = false;
                continue;
            }
            for (path, bytes) in pending {
                result.add_file(path, bytes, true, &[])?;
            }
        }
        Ok(result)
    }
    fn instruction_closure(&self, targets: &[String]) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for target in targets {
            let relative = Path::new(target);
            let mut ancestors = relative
                .parent()
                .ok_or("Invalid evidence target")?
                .ancestors()
                .collect::<Vec<_>>();
            ancestors.reverse();
            for ancestor in ancestors {
                let directory = self.root.join(ancestor);
                if !self.coverage.complete_directories.contains(&directory) {
                    return Err("Governing instruction directory coverage is incomplete".into());
                }
                for name in ["AGENTS.md", "CLAUDE.md"] {
                    let path = directory.join(name);
                    if !self.coverage.instruction_entries.contains(&path) {
                        continue;
                    }
                    let relative = path
                        .strip_prefix(&self.root)
                        .ok()
                        .and_then(canonical_relative_path)
                        .ok_or("Invalid governing instruction path")?;
                    if !self.permitted.contains(&relative) {
                        return Err("Governing instructions are inaccessible".into());
                    }
                    if seen.insert(relative.clone()) {
                        result.push(relative);
                    }
                }
            }
            if seen.insert(target.clone()) {
                result.push(target.clone());
            }
        }
        Ok(result)
    }
    fn add_file(
        &mut self,
        path: String,
        bytes: Vec<u8>,
        seed: bool,
        terms: &[String],
    ) -> Result<(), String> {
        let text = std::str::from_utf8(&bytes).map_err(|_| "Assessment file is not text")?;
        let excerpt = context::relevant_excerpt(text, terms);
        let start = text.find(&excerpt).ok_or("Invalid assessment excerpt")?;
        let sha256 = digest(&bytes);
        self.manifest.files.push(ManifestFile {
            path: path.clone(),
            sha256: sha256.clone(),
            bytes: bytes.len(),
            seed,
            excerpt_start: start,
            excerpt_end: start + excerpt.len(),
            excerpt_digest: digest(excerpt.as_bytes()),
        });
        self.facts.push(RepositoryEvidence {
            path: path.clone(),
            sha256,
            excerpt,
            observed_at: self.observed_at.clone(),
        });
        self.prefixes.insert(path, bytes);
        Ok(())
    }
    pub(crate) fn follow_up(
        &mut self,
        ids: &[usize],
        hypothesis: &str,
    ) -> Result<FollowUp, String> {
        if !self.manifest.requested_file_ids.is_empty()
            || ids.len() > 8
            || ids.iter().collect::<HashSet<_>>().len() != ids.len()
        {
            return Err("Invalid evidence selection".into());
        }
        let targets = ids
            .iter()
            .map(|id| {
                self.catalog
                    .get(*id)
                    .cloned()
                    .ok_or("Unknown evidence selection")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let paths = match self.instruction_closure(&targets) {
            Ok(paths) => paths,
            Err(_) => {
                self.manifest.requested_file_ids = ids.to_vec();
                self.manifest.instruction_coverage_complete = false;
                return Ok(FollowUp::EvidenceLimit);
            }
        };
        let mut pending = Vec::new();
        for path in paths {
            if self.prefixes.contains_key(&path) {
                continue;
            }
            match read_prefix(&self.root, &path, self.level, &self.excluded) {
                Ok(bytes) => pending.push((path, bytes)),
                Err(error) => {
                    if targets.contains(&path) {
                        return Err(error);
                    }
                    self.manifest.requested_file_ids = ids.to_vec();
                    self.manifest.instruction_coverage_complete = false;
                    return Ok(FollowUp::EvidenceLimit);
                }
            }
        }
        let total: usize = self.prefixes.values().map(Vec::len).sum();
        if total + pending.iter().map(|(_, b)| b.len()).sum::<usize>() > MAX_TOTAL
            || self.facts.len() + pending.len() > 32
        {
            self.manifest.requested_file_ids = ids.to_vec();
            return Ok(FollowUp::EvidenceLimit);
        }
        let terms = hypothesis
            .split_whitespace()
            .filter(|s| s.len() > 3)
            .take(256)
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        for (path, bytes) in pending {
            self.add_file(path, bytes, false, &terms)?;
        }
        self.manifest.requested_file_ids = ids.to_vec();
        Ok(FollowUp::Read)
    }
    pub(crate) fn bind_discovery(&mut self, output: &str) {
        self.manifest.discovery_digest = Some(digest(output.as_bytes()));
    }
    pub(crate) fn manifest(&self) -> InputManifest {
        self.manifest.clone()
    }
    pub(crate) fn restore(mut self, saved: &InputManifest) -> Result<Self, String> {
        saved.validate()?;
        if saved.redacted
            || self.manifest.context_level != saved.context_level
            || self.manifest.excluded_paths != saved.excluded_paths
            || self.manifest.catalog_digest != saved.catalog_digest
            || self.manifest.catalog_complete != saved.catalog_complete
            || self.manifest.files
                != saved
                    .files
                    .iter()
                    .filter(|f| f.seed)
                    .cloned()
                    .collect::<Vec<_>>()
        {
            return Err("Assessment repository inputs changed".into());
        }
        let targets = saved
            .requested_file_ids
            .iter()
            .map(|id| {
                self.catalog
                    .get(*id)
                    .cloned()
                    .ok_or("Unknown saved selection")
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Revalidate an all-or-nothing insufficient-evidence result without
        // reconstructing any hypothesis prose or supplying a partial read set.
        let mut limited = self.clone();
        if limited.follow_up(&saved.requested_file_ids, "")? == FollowUp::EvidenceLimit {
            limited.manifest.discovery_digest = saved.discovery_digest.clone();
            if limited.manifest == *saved {
                return Ok(limited);
            }
            return Err("Assessment evidence limit inputs changed".into());
        }
        let expected = self
            .instruction_closure(&targets)?
            .into_iter()
            .filter(|path| !self.prefixes.contains_key(path))
            .collect::<HashSet<_>>();
        let actual = saved
            .files
            .iter()
            .filter(|f| !f.seed)
            .map(|f| f.path.clone())
            .collect::<HashSet<_>>();
        if expected != actual {
            return Err("Assessment instruction selection changed".into());
        }
        for file in saved.files.iter().filter(|f| !f.seed) {
            let bytes = limited
                .prefixes
                .get(&file.path)
                .cloned()
                .ok_or("Assessment selection is not permitted")?;
            if bytes.len() != file.bytes || digest(&bytes) != file.sha256 {
                return Err("Assessment evidence changed".into());
            }
            let text = std::str::from_utf8(&bytes).map_err(|_| "Assessment file is not text")?;
            let excerpt = text
                .get(file.excerpt_start..file.excerpt_end)
                .ok_or("Invalid saved excerpt")?;
            if excerpt.chars().count() > 2000 || digest(excerpt.as_bytes()) != file.excerpt_digest {
                return Err("Assessment excerpt changed".into());
            }
            self.facts.push(RepositoryEvidence {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                excerpt: excerpt.into(),
                observed_at: self.observed_at.clone(),
            });
            self.prefixes.insert(file.path.clone(), bytes);
        }
        for id in &saved.requested_file_ids {
            if self.catalog.get(*id).is_none() {
                return Err("Unknown saved evidence selection".into());
            }
        }
        self.manifest = saved.clone();
        Ok(self)
    }
}
fn canonical_relative_path(path: &Path) -> Option<String> {
    path.components()
        .map(|part| match part {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}
fn read_prefix(
    root: &Path,
    relative: &str,
    level: ContextLevel,
    excluded: &[String],
) -> Result<Vec<u8>, String> {
    if !context::fact_is_permitted(relative, level, excluded) {
        return Err("Assessment evidence access denied".into());
    }
    let path = root.join(relative);
    let repository = git2::Repository::discover(root).ok();
    if repository.as_ref().is_some_and(|repo| {
        repo.workdir()
            .and_then(|p| p.canonicalize().ok())
            .and_then(|p| path.strip_prefix(p).ok().map(Path::to_path_buf))
            .is_none_or(|p| repo.status_should_ignore(&p).unwrap_or(true))
    }) {
        return Err("Assessment evidence is ignored".into());
    }
    let mut bytes = Vec::new();
    context::open_context_file(&path)
        .map_err(|_| "Assessment evidence cannot be safely opened")?
        .take(MAX_PREFIX as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Assessment evidence cannot be read")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "Assessment evidence is not text")?;
    let lower = text.to_ascii_lowercase();
    if [
        "private_key",
        "client_secret",
        "api_key",
        "apikey",
        "access_token",
        "-----begin",
    ]
    .iter()
    .any(|m| lower.contains(m))
    {
        return Err("Assessment evidence contains sensitive data".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::runtime::ContextLevel;
    use std::fs;

    #[test]
    fn repository_seeds_reserve_workflow_evidence_and_catalog_followup() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Unit tests are required").unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts":{"test":"echo skipped"}}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join(".github/workflows/ci.yml"),
            "steps: unit checks",
        )
        .unwrap();
        fs::write(dir.path().join("src/check.ts"), "test fixture").unwrap();
        let mut context =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        assert!(context
            .facts
            .iter()
            .any(|f| f.path == ".github/workflows/ci.yml"));
        let id = context
            .catalog
            .iter()
            .position(|p| p == "src/check.ts")
            .unwrap();
        context.follow_up(&[id], "unit checks").unwrap();
        assert!(context.facts.iter().any(|f| f.path == "src/check.ts"));
        assert!(context
            .manifest()
            .files
            .iter()
            .any(|f| f.path == "src/check.ts"));
        let again =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "later").unwrap();
        assert_eq!(context.catalog, again.catalog);
        assert!(RepositoryContext::collect(
            dir.path(),
            ContextLevel::SessionObservations,
            &[],
            "now"
        )
        .is_err());
    }

    #[test]
    fn followup_revalidates_permitted_paths_and_preserves_shared_budget() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "a".repeat(8192)).unwrap();
        fs::write(dir.path().join("README.md"), "r".repeat(8192)).unwrap();
        fs::write(dir.path().join("package.json"), "p".repeat(8192)).unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        for n in 0..8 {
            fs::write(dir.path().join(format!("src/{n}.ts")), "x".repeat(8192)).unwrap();
        }
        let mut context =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        let ids: Vec<_> = context
            .catalog
            .iter()
            .enumerate()
            .filter(|(_, p)| p.starts_with("src/"))
            .map(|(i, _)| i)
            .collect();
        let before = context.facts.clone();
        assert_eq!(
            context.follow_up(&ids, "test").unwrap(),
            FollowUp::EvidenceLimit
        );
        assert_eq!(context.facts, before, "no partial requested set supplied");
        context.bind_discovery("validated budget-limited discovery");
        let saved = context.manifest();
        let restored = RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now")
            .unwrap()
            .restore(&saved)
            .unwrap();
        assert_eq!(restored.manifest(), saved);

        fs::write(dir.path().join("src/0.ts"), "api_key=private").unwrap();
        assert!(context.follow_up(&ids[..1], "test").is_err());
        assert_eq!(context.facts, before);
    }

    #[test]
    fn catalog_filters_ignored_excluded_credentials_and_tracks_uncited_changes() {
        let dir = tempfile::tempdir().unwrap();
        let _git = git2::Repository::init(dir.path()).unwrap();
        for (name, text) in [
            ("README.md", "guide"),
            ("ignored.md", "ignore"),
            ("excluded.md", "exclude"),
            ("token.md", "api_key=private"),
            (".gitignore", "ignored.md"),
        ] {
            fs::write(dir.path().join(name), text).unwrap();
        }
        let context = RepositoryContext::collect(
            dir.path(),
            ContextLevel::WorkflowContext,
            &["excluded.md".into()],
            "now",
        )
        .unwrap();
        for name in ["ignored.md", "excluded.md", "token.md"] {
            assert!(!context.catalog.iter().any(|p| p == name));
        }
        let before = context.manifest();
        fs::write(dir.path().join("new.md"), "new uncited input").unwrap();
        let after = RepositoryContext::collect(
            dir.path(),
            ContextLevel::WorkflowContext,
            &["excluded.md".into()],
            "now",
        )
        .unwrap()
        .manifest();
        assert_ne!(before.catalog_digest, after.catalog_digest);
    }
}

#[cfg(test)]
mod safety_tests {
    use super::*;
    use std::fs;
    #[test]
    fn catalog_and_seed_caps_keep_root_workflow_ahead_of_history() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("archive")).unwrap();
        for n in 0..600 {
            fs::write(
                dir.path().join(format!("archive/{n:03}.md")),
                "historical report",
            )
            .unwrap();
        }
        fs::write(dir.path().join("AGENTS.md"), "Current required checks").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        let context =
            RepositoryContext::collect(dir.path(), ContextLevel::WorkflowContext, &[], "now")
                .unwrap();
        assert!(!context.manifest().catalog_complete);
        assert!(context.catalog.len() <= 512);
        assert!(serde_json::to_vec(&context.catalog).unwrap().len() <= 16384);
        assert!(context.facts.len() <= 16);
        assert!(context.facts.iter().any(|f| f.path == "AGENTS.md"));
        assert!(context.facts.iter().any(|f| f.path == "package.json"));
        assert!(
            context
                .manifest()
                .files
                .iter()
                .map(|f| f.bytes)
                .sum::<usize>()
                <= 24576
        );
    }
    #[test]
    fn restored_selection_tracks_uncited_prefixes_and_cannot_reopen_narrowed_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Unit suite is required").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/test.ts"),
            format!("{}\nunit test evidence", "intro\n".repeat(500)),
        )
        .unwrap();
        let mut context =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        let id = context
            .catalog
            .iter()
            .position(|p| p == "src/test.ts")
            .unwrap();
        context.follow_up(&[id], "unit test evidence").unwrap();
        context.bind_discovery("validated response");
        let saved = context.manifest();
        let restored =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "later")
                .unwrap()
                .restore(&saved)
                .unwrap();
        assert_eq!(restored.facts.last().unwrap().excerpt, "unit test evidence");
        assert!(RepositoryContext::collect(
            dir.path(),
            ContextLevel::WorkflowContext,
            &[],
            "later"
        )
        .unwrap()
        .restore(&saved)
        .is_err());
        fs::write(dir.path().join("AGENTS.md"), "Changed uncited instructions").unwrap();
        assert!(
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "later")
                .unwrap()
                .restore(&saved)
                .is_err()
        );
    }
    #[cfg(unix)]
    #[test]
    fn followup_rejects_catalog_file_or_parent_replaced_with_symlink() {
        use std::os::unix::fs::symlink;
        for parent in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            fs::create_dir(dir.path().join("src")).unwrap();
            fs::write(dir.path().join("src/test.ts"), "allowed test evidence").unwrap();
            fs::write(outside.path().join("test.ts"), "private outside evidence").unwrap();
            let mut context =
                RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now")
                    .unwrap();
            let id = context
                .catalog
                .iter()
                .position(|p| p == "src/test.ts")
                .unwrap();
            fs::remove_file(dir.path().join("src/test.ts")).unwrap();
            if parent {
                fs::remove_dir(dir.path().join("src")).unwrap();
                symlink(outside.path(), dir.path().join("src")).unwrap();
            } else {
                symlink(
                    outside.path().join("test.ts"),
                    dir.path().join("src/test.ts"),
                )
                .unwrap();
            }
            assert!(context.follow_up(&[id], "test evidence").is_err());
            assert!(context.facts.is_empty());
        }
    }
}

#[cfg(test)]
mod instruction_contract_tests {
    use super::*;
    use std::fs;
    #[test]
    fn followup_automatically_supplies_ancestor_instructions_outside_initial_seed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Root workflow rule").unwrap();
        for n in 0..20 {
            fs::create_dir(dir.path().join(format!("a{n:02}"))).unwrap();
            fs::write(
                dir.path().join(format!("a{n:02}/AGENTS.md")),
                "Other scoped instructions",
            )
            .unwrap();
        }
        fs::create_dir_all(dir.path().join("z/child")).unwrap();
        fs::write(
            dir.path().join("z/AGENTS.md"),
            "Governing parent constraint",
        )
        .unwrap();
        fs::write(
            dir.path().join("z/child/CLAUDE.md"),
            "Governing child constraint",
        )
        .unwrap();
        fs::write(dir.path().join("z/child/test.ts"), "related test evidence").unwrap();
        let mut context =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        assert!(!context.facts.iter().any(|f| f.path == "z/AGENTS.md"));
        let id = context
            .catalog
            .iter()
            .position(|p| p == "z/child/test.ts")
            .unwrap();
        assert_eq!(
            context.follow_up(&[id], "related test evidence").unwrap(),
            FollowUp::Read
        );
        for path in ["z/AGENTS.md", "z/child/CLAUDE.md", "z/child/test.ts"] {
            assert!(
                context.facts.iter().any(|f| f.path == path),
                "missing governing evidence {path}"
            );
        }
        let saved = context.manifest();
        let restored =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "later")
                .unwrap()
                .restore(&saved)
                .unwrap();
        assert_eq!(restored.manifest(), saved);
    }
    #[test]
    fn inaccessible_governing_instruction_prevents_followup_without_partial_evidence() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/AGENTS.md"),
            "Excluded governing instructions",
        )
        .unwrap();
        fs::write(dir.path().join("src/test.ts"), "test evidence").unwrap();
        let mut context = RepositoryContext::collect(
            dir.path(),
            ContextLevel::SourceCode,
            &["src/AGENTS.md".into()],
            "now",
        )
        .unwrap();
        let id = context
            .catalog
            .iter()
            .position(|p| p == "src/test.ts")
            .unwrap();
        let before = context.facts.clone();
        assert_eq!(
            context.follow_up(&[id], "test evidence").unwrap(),
            FollowUp::EvidenceLimit
        );
        assert_eq!(context.facts, before);
        context.bind_discovery("validated inaccessible instruction discovery");
        let saved = context.manifest();
        assert!(RepositoryContext::collect(
            dir.path(),
            ContextLevel::SourceCode,
            &["src/AGENTS.md".into()],
            "now"
        )
        .unwrap()
        .restore(&saved)
        .is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn invalid_workspace_encoding_is_unavailable_and_native_backslashes_do_not_alias_paths() {
        use std::os::unix::ffi::OsStringExt;
        let dir = tempfile::tempdir().unwrap();
        let invalid = dir
            .path()
            .join(std::ffi::OsString::from_vec(vec![b'w', 255]));
        let native_invalid_paths_supported = fs::create_dir(&invalid).is_ok();
        if native_invalid_paths_supported {
            fs::write(invalid.join("README.md"), "workflow rule").unwrap();
        }
        assert!(
            RepositoryContext::collect(&invalid, ContextLevel::WorkflowContext, &[], "now")
                .err()
                .unwrap()
                .contains("UTF-8")
        );
        assert!(
            canonical_relative_path(Path::new(&std::ffi::OsString::from_vec(vec![b'f', 255])))
                .is_none()
        );
        fs::write(dir.path().join("control\nname.md"), "omitted path").unwrap();
        fs::write(dir.path().join(r"native\path.md"), "literal native name").unwrap();
        let context =
            RepositoryContext::collect(dir.path(), ContextLevel::WorkflowContext, &[], "now")
                .unwrap();
        assert!(context.catalog.iter().any(|p| p == r"native\path.md"));
        assert!(context.facts.iter().any(|f| f.path == r"native\path.md"));
        assert!(
            !context.manifest().catalog_complete,
            "invalid child paths make coverage partial"
        );
    }
}
