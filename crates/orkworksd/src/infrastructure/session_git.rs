use std::path::PathBuf;
use crate::application::session::ports::GitDetector;

pub struct RealGitDetector;

impl GitDetector for RealGitDetector {
    fn detect(&self, path: &PathBuf) -> Option<crate::git::GitContext> {
        Some(crate::git::detect(path))
    }
}
