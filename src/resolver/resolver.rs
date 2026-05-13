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
                .or_else(|| self.resolve_common_texture_locations(source, &texture_path))
                .or_else(|| self.resolve_plain_path(&texture_path))
                .unwrap_or_else(|| source.resolve(&texture_path)),
            Some(source) => self
                .resolve_plain_path(&texture_path)
                .or_else(|| self.resolve_common_texture_locations(source, &texture_path))
                .or_else(|| self.resolve_against_source(source, &texture_path))
                .unwrap_or_else(|| source.resolve(&texture_path)),
            None => self
                .resolve_plain_path(&texture_path)
                .or_else(|| {
                    let source = PmxSource::folder(".");
                    self.resolve_common_texture_locations(&source, &texture_path)
                })
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

    fn resolve_common_texture_locations(
        &self,
        source: &PmxSource,
        texture_path: &Path,
    ) -> Option<PathBuf> {
        if texture_path.components().count() != 1 {
            return None;
        }

        let file_name = texture_path.file_name()?;
        for directory in common_texture_directories() {
            let candidate = Path::new(directory).join(file_name);
            let resolved = source.resolve(&candidate);
            if resolved.exists() {
                return Some(resolved);
            }

            if self.settings.allow_case_fallbacks {
                if let Some(found) = source.resolve_case_insensitive(&candidate) {
                    return Some(found);
                }
            }
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

fn common_texture_directories() -> &'static [&'static str] {
    &["Texture", "texture", "textures", "tex", "spa", "sp", "toon"]
}

#[cfg(test)]
mod tests {
    use super::{PmxResolver, PmxResolverSettings};
    use crate::source::PmxSource;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn resolves_bare_texture_names_from_common_mmd_directories_case_insensitively() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("bevy_pmx_resolver_{unique}"));
        let model_root = root.join("Model");
        let texture_root = model_root.join("Texture");
        fs::create_dir_all(&texture_root).expect("should create texture directory");
        fs::write(texture_root.join("Face.PNG"), b"").expect("should create texture file");

        let resolver = PmxResolver::with_settings(PmxResolverSettings::default());
        let source = PmxSource::folder(PathBuf::from(&model_root));
        let resolved = resolver.resolve_texture_path(Some(&source), "face.png");

        assert!(resolved.resolved.exists());
        assert_eq!(
            fs::canonicalize(&resolved.resolved).expect("resolved path should canonicalize"),
            fs::canonicalize(texture_root.join("Face.PNG"))
                .expect("expected texture path should canonicalize")
        );
    }
}
