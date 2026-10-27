//! Repository-owned evidence selection for manual assessment only.
use super::{context, runtime::ContextLevel, RepositoryEvidence};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};

pub(super) const COLLECTOR_VERSION: u8 = 3;
const MAX_PREFIX: usize = 8192;
const MAX_TOTAL: usize = 65536;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InputManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
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
        if !matches!(self.collector_version, 2 | COLLECTOR_VERSION)
            || match self.purpose.as_deref() {
                None => self.prompt_version != super::assessment::ASSESSMENT_PROMPT_VERSION,
                Some("readiness") => self.prompt_version != "taskmaster-readiness-v1",
                _ => true,
            }
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
    unavailable_inputs: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FollowUp {
    Read,
    EvidenceLimit,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RevalidationError {
    Changed,
    Unavailable,
}

impl RepositoryContext {
    pub(crate) fn collect_for_readiness(
        workspace: &Path,
        level: ContextLevel,
        excluded: &[String],
        observed_at: &str,
    ) -> Result<Self, String> {
        let mut context = Self::collect(workspace, level, excluded, observed_at)?;
        context.manifest.purpose = Some("readiness".into());
        context.manifest.prompt_version = "taskmaster-readiness-v1".into();
        Ok(context)
    }
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
        paths.sort_by_cached_key(|p| seed_priority(p.strip_prefix(&root).unwrap_or(p)));
        let mut catalog = Vec::new();
        let mut permitted = HashSet::new();
        let mut size = 2;
        let mut unavailable_inputs = false;
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
            // Keep permission metadata for governing instructions after the cap,
            // but never open contents that cannot enter the catalog.
            if catalog.len() == 512 {
                complete = false;
                continue;
            }
            let encoded = serde_json::to_string(&relative).map_err(|_| "Invalid catalog path")?;
            if size + encoded.len() + 1 > 16384 {
                complete = false;
                continue;
            }
            // A catalog entry is permission data, not proof of absence. Vet the
            // bounded prefix without retaining or supplying unselected contents.
            if read_prefix(&root, &relative, level, excluded).is_err() {
                unavailable_inputs = true;
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
            unavailable_inputs,
            manifest: InputManifest {
                purpose: None,
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
        seeds.sort_by_cached_key(|p| seed_priority(Path::new(p)));
        for path in seeds {
            // Scoped instructions accompany their selected workflow targets through
            // instruction_closure; unrelated instruction files cannot consume the
            // seed allowance or make coverage incomplete for supplied targets.
            if Path::new(&path).components().count() > 1
                && matches!(
                    Path::new(&path).file_name().and_then(|n| n.to_str()),
                    Some("AGENTS.md" | "CLAUDE.md")
                )
            {
                continue;
            }
            let total: usize = result.prefixes.values().map(Vec::len).sum();
            if result.facts.len() == 16 || total == 24576 {
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
            if denied {
                result.manifest.instruction_coverage_complete = false;
                continue;
            }
            if result.facts.len() + pending.len() > 16
                || total + pending.iter().map(|(_, b)| b.len()).sum::<usize>() > 24576
            {
                let instructions: Vec<_> = pending
                    .iter()
                    .filter(|(candidate, _)| {
                        candidate != &path
                            || matches!(
                                Path::new(candidate)
                                    .file_name()
                                    .and_then(|name| name.to_str()),
                                Some("AGENTS.md" | "CLAUDE.md")
                            )
                    })
                    .collect();
                if result.facts.len() + instructions.len() > 16
                    || total
                        + instructions
                            .iter()
                            .map(|(_, bytes)| bytes.len())
                            .sum::<usize>()
                        > 24576
                {
                    result.manifest.instruction_coverage_complete = false;
                    continue;
                }
                // Ordinary seed truncation does not leave a supplied target
                // without its instructions. Follow-up still has its shared budget.
                break;
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
    pub(crate) fn restore_for_readiness(
        self,
        saved: &InputManifest,
    ) -> Result<Self, RevalidationError> {
        saved
            .validate()
            .map_err(|_| RevalidationError::Unavailable)?;
        if self.unavailable_inputs {
            return Err(RevalidationError::Unavailable);
        }
        let mut changed = false;
        // Classify disappeared paths without opening evidence outside this catalog.
        for file in saved
            .files
            .iter()
            .filter(|file| !self.permitted.contains(&file.path))
        {
            match std::fs::symlink_metadata(self.root.join(&file.path)) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => changed = true,
                _ => return Err(RevalidationError::Unavailable),
            }
        }
        if changed
            || self.manifest.collector_version != saved.collector_version
            || self.manifest.context_level != saved.context_level
            || self.manifest.excluded_paths != saved.excluded_paths
            || self.manifest.catalog_digest != saved.catalog_digest
            || self.manifest.catalog_complete != saved.catalog_complete
            || self.manifest.files
                != saved
                    .files
                    .iter()
                    .filter(|file| file.seed)
                    .cloned()
                    .collect::<Vec<_>>()
        {
            return Err(RevalidationError::Changed);
        }
        // The original selection still fits the current catalog and permission set.
        for file in &saved.files {
            let bytes = read_prefix(&self.root, &file.path, self.level, &self.excluded)
                .map_err(|_| RevalidationError::Unavailable)?;
            changed |= bytes.len() != file.bytes || digest(&bytes) != file.sha256;
        }
        if changed {
            return Err(RevalidationError::Changed);
        }
        self.restore(saved)
            .map_err(|_| RevalidationError::Unavailable)
    }
    pub(crate) fn restore(mut self, saved: &InputManifest) -> Result<Self, String> {
        saved.validate()?;
        if saved.redacted
            || self.manifest.collector_version != saved.collector_version
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
/// Operational configuration supplies discovery with evidence to investigate;
/// governing instructions are added atomically before each selected target.
fn seed_priority(path: &Path) -> (u8, PathBuf) {
    let (priority, _, relative) = context::context_priority(path, &[]);
    let name = path.file_name().and_then(|name| name.to_str());
    let priority = if priority == 5 {
        7 // Historical guidance remains last.
    } else if matches!(
        name,
        Some("Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod")
    ) {
        0
    } else if path.starts_with(".github/workflows") {
        1
    } else {
        priority + 2
    };
    (priority, relative)
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
#[cfg(test)]
thread_local! {
    static OBSERVED_PREFIX_READS: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
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
    #[cfg(test)]
    OBSERVED_PREFIX_READS.with(|reads| {
        if let Some(reads) = reads.borrow_mut().as_mut() {
            reads.push(relative.to_owned());
        }
    });
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
    fn readiness_revalidation_distinguishes_changes_from_unavailable_follow_up() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(directory.path().join("README.md"), "Repository guide").unwrap();
        let source = directory.path().join("src/main.rs");
        std::fs::write(&source, "fn main() {}\n").unwrap();
        let collect = || {
            RepositoryContext::collect_for_readiness(
                directory.path(),
                ContextLevel::SourceCode,
                &[],
                "now",
            )
            .unwrap()
        };
        let mut initial = collect();
        let id = initial
            .catalog
            .iter()
            .position(|path| path == "src/main.rs")
            .unwrap();
        initial.follow_up(&[id], "source").unwrap();
        initial.bind_discovery("selection");
        let saved = initial.manifest();
        assert!(collect().restore_for_readiness(&saved).is_ok());
        std::fs::write(&source, "fn main() { println!(\"changed\"); }\n").unwrap();
        assert!(matches!(
            collect().restore_for_readiness(&saved),
            Err(RevalidationError::Changed)
        ));
        std::fs::write(&source, [0xff, 0xfe]).unwrap();
        assert!(matches!(
            collect().restore_for_readiness(&saved),
            Err(RevalidationError::Unavailable)
        ));
        std::fs::remove_file(&source).unwrap();
        assert!(matches!(
            collect().restore_for_readiness(&saved),
            Err(RevalidationError::Changed)
        ));
        #[cfg(unix)]
        {
            let outside = tempfile::NamedTempFile::new().unwrap();
            std::os::unix::fs::symlink(outside.path(), &source).unwrap();
            assert!(matches!(
                collect().restore_for_readiness(&saved),
                Err(RevalidationError::Unavailable)
            ));
        }
    }

    #[test]
    fn linked_guidance_reaches_the_catalog_before_large_skill_trees_exhaust_the_budget() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".agents/skills")).unwrap();
        fs::create_dir_all(dir.path().join("docs/agents")).unwrap();
        fs::write(
            dir.path().join("AGENTS.md"),
            "See [architecture](docs/agents/architecture.md) and [domain](docs/agents/domain.md).",
        )
        .unwrap();
        fs::write(
            dir.path().join("docs/agents/architecture.md"),
            "The inventory module owns stock transitions.",
        )
        .unwrap();
        fs::write(
            dir.path().join("docs/agents/domain.md"),
            "Stock quantities cannot be negative.",
        )
        .unwrap();
        for i in 0..2100 {
            fs::write(
                dir.path()
                    .join(format!(".agents/skills/distractor-{i:04}.md")),
                "Generic skill instructions",
            )
            .unwrap();
        }
        let mut context = RepositoryContext::collect_for_readiness(
            dir.path(),
            ContextLevel::SourceCode,
            &[],
            "now",
        )
        .unwrap();
        let ids = ["docs/agents/architecture.md", "docs/agents/domain.md"]
            .iter()
            .map(|path| {
                context
                    .catalog
                    .iter()
                    .position(|entry| entry == path)
                    .expect("linked guidance was starved from the bounded catalog")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            context.follow_up(&ids, "architecture domain").unwrap(),
            FollowUp::Read
        );
        assert!(context
            .facts
            .iter()
            .any(|fact| fact.excerpt.contains("cannot be negative")));
        let excluded = RepositoryContext::collect_for_readiness(
            dir.path(),
            ContextLevel::SourceCode,
            &["docs".into()],
            "later",
        )
        .unwrap();
        assert!(excluded
            .catalog
            .iter()
            .all(|path| !path.starts_with("docs/")));
    }

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
    fn scoped_instructions_do_not_starve_operational_workflow_seeds() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "a".repeat(8192)).unwrap();
        fs::write(dir.path().join("README.md"), "r".repeat(8192)).unwrap();
        for component in ["apps/desktop", "crates/service", "tests/fixtures"] {
            fs::create_dir_all(dir.path().join(component)).unwrap();
            fs::write(
                dir.path().join(component).join("AGENTS.md"),
                "Scoped instructions. ".repeat(160),
            )
            .unwrap();
        }
        fs::write(
            dir.path().join("apps/desktop/package.json"),
            format!(
                r#"{{"scripts":{{"test":"echo skipped"}},"description":"{}"}}"#,
                "x".repeat(4000)
            ),
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
        fs::write(dir.path().join(".github/workflows/ci.yml"), "unit checks").unwrap();

        let context =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        for expected in [
            "AGENTS.md",
            "apps/desktop/AGENTS.md",
            "apps/desktop/package.json",
            ".github/workflows/ci.yml",
        ] {
            assert!(
                context.facts.iter().any(|fact| fact.path == expected),
                "missing operational seed or governing instructions: {expected}"
            );
        }
        let manifest = context.manifest();
        assert!(manifest.instruction_coverage_complete);
        assert!(manifest.files.len() <= 16);
        assert!(manifest.files.iter().map(|file| file.bytes).sum::<usize>() <= 24576);
        context.clone().restore(&manifest).unwrap();
    }

    #[test]
    fn normal_seed_byte_truncation_preserves_instruction_coverage_and_followup() {
        for workflow_bytes in [8192, 8000] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("AGENTS.md"), "a".repeat(8192)).unwrap();
            fs::write(dir.path().join("README.md"), "r".repeat(workflow_bytes)).unwrap();
            fs::write(dir.path().join("package.json"), "p".repeat(workflow_bytes)).unwrap();
            fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
            fs::write(
                dir.path().join(".github/workflows/ci.yml"),
                "c".repeat(1024),
            )
            .unwrap();
            fs::create_dir(dir.path().join("src")).unwrap();
            fs::write(dir.path().join("src/check.ts"), "related test evidence").unwrap();
            let mut context =
                RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now")
                    .unwrap();
            assert!(
                context.manifest().instruction_coverage_complete,
                "ordinary seed truncation is not a missing governing instruction"
            );
            assert!(context
                .facts
                .iter()
                .any(|f| f.path == ".github/workflows/ci.yml"));
            assert!(!context.facts.iter().any(|f| f.path == "README.md"));
            let id = context
                .catalog
                .iter()
                .position(|p| p == "src/check.ts")
                .unwrap();
            assert_eq!(
                context.follow_up(&[id], "test evidence").unwrap(),
                FollowUp::Read
            );
            assert!(context.manifest().instruction_coverage_complete);
            context.bind_discovery("validated selection after bounded seeding");
            let saved = context.manifest();
            let restored =
                RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "later")
                    .unwrap()
                    .restore(&saved)
                    .unwrap();
            assert_eq!(restored.manifest(), saved);
        }
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
    fn exactly_full_root_traversal_preserves_instruction_coverage() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "Required repository checks").unwrap();
        fs::write(dir.path().join("README.md"), "Current workflow").unwrap();
        for index in 0..1998 {
            fs::write(
                dir.path().join(format!("source-{index:04}.ts")),
                "export const value = 1;",
            )
            .unwrap();
        }
        let exact =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        assert!(exact.manifest().instruction_coverage_complete);
        assert!(exact.facts.iter().any(|fact| fact.path == "AGENTS.md"));
        assert!(
            !exact.manifest().catalog_complete,
            "catalog still has its own smaller cap"
        );
        fs::write(dir.path().join("one-more.ts"), "export const value = 2;").unwrap();
        let partial =
            RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        assert!(!partial.manifest().instruction_coverage_complete);
    }

    #[test]
    fn catalog_capacity_does_not_read_unsupplied_source_prefixes() {
        for long_names in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("AGENTS.md"), "Required unit coverage").unwrap();
            fs::create_dir(dir.path().join("src")).unwrap();
            for index in 0..700 {
                let padding = if long_names {
                    "x".repeat(110)
                } else {
                    String::new()
                };
                fs::write(
                    dir.path().join(format!("src/{index:03}{padding}.ts")),
                    "export const value = 1;",
                )
                .unwrap();
            }
            OBSERVED_PREFIX_READS.with(|reads| *reads.borrow_mut() = Some(Vec::new()));
            let collected =
                RepositoryContext::collect(dir.path(), ContextLevel::SourceCode, &[], "now");
            let reads = OBSERVED_PREFIX_READS.with(|reads| reads.borrow_mut().take().unwrap());
            let collected = collected.unwrap();
            assert!(!collected.manifest().catalog_complete);
            assert!(collected.manifest().instruction_coverage_complete);
            assert!(
                reads.iter().all(|path| collected.catalog.contains(path)),
                "read prefixes outside capped catalog: {} reads, {} entries",
                reads.len(),
                collected.catalog.len()
            );
            assert!(collected.facts.iter().any(|fact| fact.path == "AGENTS.md"));
        }
    }

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
