use std::path::{Path, PathBuf};

use crate::source::{PmxFolderSource, PmxSource, PmxSourceLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxResolvedPath {
    pub original: String,
    pub location: PmxSourceLocation,
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
            return PmxResolvedPath::new(original, PmxSourceLocation::Disk(texture_path));
        }

        let location = match source {
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
                .unwrap_or_else(|| PmxSourceLocation::Disk(texture_path.clone())),
        };

        PmxResolvedPath::new(original, location)
    }

    fn resolve_against_source(
        &self,
        source: &PmxSource,
        texture_path: &Path,
    ) -> Option<PmxSourceLocation> {
        let resolved = source.resolve(texture_path);
        if source.contains_location(&resolved) {
            return Some(resolved);
        }

        if self.settings.allow_case_fallbacks {
            return source.resolve_case_insensitive(texture_path);
        }

        None
    }

    fn resolve_plain_path(&self, texture_path: &Path) -> Option<PmxSourceLocation> {
        if texture_path.exists() {
            return Some(PmxSourceLocation::Disk(texture_path.to_path_buf()));
        }

        if self.settings.allow_case_fallbacks {
            return PmxFolderSource::new(".")
                .resolve_case_insensitive(texture_path)
                .map(PmxSourceLocation::Disk);
        }

        None
    }

    fn resolve_common_texture_locations(
        &self,
        source: &PmxSource,
        texture_path: &Path,
    ) -> Option<PmxSourceLocation> {
        if texture_path.components().count() != 1 {
            return None;
        }

        let file_name = texture_path.file_name()?;
        for directory in common_texture_directories() {
            let candidate = Path::new(directory).join(file_name);
            let resolved = source.resolve(&candidate);
            if source.contains_location(&resolved) {
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
    pub fn new(original: impl Into<String>, location: impl Into<PmxSourceLocation>) -> Self {
        Self {
            original: original.into(),
            location: location.into(),
        }
    }

    pub fn location(&self) -> &PmxSourceLocation {
        &self.location
    }

    pub fn source_location(&self) -> &PmxSourceLocation {
        &self.location
    }

    pub fn resolved_path(&self) -> &Path {
        self.location.as_path()
    }

    pub fn relative_to(&self, root: impl AsRef<Path>) -> Option<PathBuf> {
        match &self.location {
            PmxSourceLocation::Disk(path) => {
                path.strip_prefix(root.as_ref()).ok().map(Path::to_path_buf)
            }
            PmxSourceLocation::Zip { entry, .. } => Some(PathBuf::from(entry)),
        }
    }
}

impl From<PathBuf> for PmxSourceLocation {
    fn from(value: PathBuf) -> Self {
        Self::Disk(value)
    }
}

impl From<&Path> for PmxSourceLocation {
    fn from(value: &Path) -> Self {
        Self::Disk(value.to_path_buf())
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
    use super::{PmxResolvedPath, PmxResolver, PmxResolverSettings};
    use crate::source::{PmxSource, PmxSourceLocation};
    use std::{
        fs,
        path::{Path, PathBuf},
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

        assert!(resolved.resolved_path().exists());
        assert_eq!(
            fs::canonicalize(resolved.resolved_path()).expect("resolved path should canonicalize"),
            fs::canonicalize(texture_root.join("Face.PNG"))
                .expect("expected texture path should canonicalize")
        );
    }

    #[test]
    fn computes_texture_path_relative_to_the_source_root() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let source_root = std::env::temp_dir().join(format!("bevy_pmx_resolver_root_{unique}"));
        let resolved =
            PmxResolvedPath::new("face.png", source_root.join("Texture").join("Face.PNG"));

        let relative = resolved
            .relative_to(&source_root)
            .expect("should strip the source root");

        assert_eq!(relative, PathBuf::from("Texture/Face.PNG"));
    }

    #[test]
    fn tracks_zip_locations_without_forcing_them_into_paths() {
        let resolved = PmxResolvedPath::new(
            "face.png",
            PmxSourceLocation::Zip {
                archive: PathBuf::from("model.zip"),
                entry: "Texture/Face.PNG".to_owned(),
            },
        );

        assert_eq!(resolved.resolved_path(), Path::new("Texture/Face.PNG"));
        assert_eq!(
            resolved.location(),
            &PmxSourceLocation::Zip {
                archive: PathBuf::from("model.zip"),
                entry: "Texture/Face.PNG".to_owned(),
            }
        );
    }
}
