use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxResolvedPath {
    pub original: String,
    pub resolved: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxResolverSettings {
    pub prefer_model_directory: bool,
    pub allow_case_fallbacks: bool,
}

impl Default for PmxResolverSettings {
    fn default() -> Self {
        Self {
            prefer_model_directory: true,
            allow_case_fallbacks: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PmxResolver {
    pub settings: PmxResolverSettings,
}

impl PmxResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_settings(settings: PmxResolverSettings) -> Self {
        Self { settings }
    }
}

impl PmxResolvedPath {
    pub fn new(original: impl Into<String>, resolved: impl Into<PathBuf>) -> Self {
        Self {
            original: original.into(),
            resolved: resolved.into(),
        }
    }

    pub fn resolved_path(&self) -> &Path {
        &self.resolved
    }
}
