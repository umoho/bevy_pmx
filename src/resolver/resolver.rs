use std::path::{Path, PathBuf};

use crate::source::{PmxFolderSource, PmxSource};

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

    pub fn resolve_texture_path(
        &self,
        source: Option<&PmxSource>,
        texture: impl AsRef<str>,
    ) -> PmxResolvedPath {
        let original = texture.as_ref().to_owned();
        let texture_path = normalize_texture_path(texture.as_ref());

        if texture_path.is_absolute() {
            return PmxResolvedPath::new(original, texture_path);
        }

        let resolved = match source {
            Some(source) if self.settings.prefer_model_directory => self
                .resolve_against_source(source, &texture_path)
                .or_else(|| self.resolve_plain_path(&texture_path))
                .unwrap_or_else(|| source.resolve(&texture_path)),
            Some(source) => self
                .resolve_plain_path(&texture_path)
                .or_else(|| self.resolve_against_source(source, &texture_path))
                .unwrap_or_else(|| source.resolve(&texture_path)),
            None => self
                .resolve_plain_path(&texture_path)
                .unwrap_or_else(|| texture_path.clone()),
        };

        PmxResolvedPath::new(original, resolved)
    }

    fn resolve_against_source(&self, source: &PmxSource, texture_path: &Path) -> Option<PathBuf> {
        let resolved = source.resolve(texture_path);
        if resolved.exists() {
            return Some(resolved);
        }

        if self.settings.allow_case_fallbacks {
            return source.resolve_case_insensitive(texture_path);
        }

        None
    }

    fn resolve_plain_path(&self, texture_path: &Path) -> Option<PathBuf> {
        if texture_path.exists() {
            return Some(texture_path.to_path_buf());
        }

        if self.settings.allow_case_fallbacks {
            return PmxFolderSource::new(".").resolve_case_insensitive(texture_path);
        }

        None
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

fn normalize_texture_path(value: &str) -> PathBuf {
    PathBuf::from(value.trim().replace('\\', "/"))
}
