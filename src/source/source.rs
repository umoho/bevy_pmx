use std::{
    borrow::Cow,
    fmt, fs,
    io::{self, Cursor, Read},
    path::{Component, Path, PathBuf},
};

use encoding_rs::{GBK, SHIFT_JIS};
use zip::{ZipArchive, read::ZipFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxFolderSource {
    pub root: PathBuf,
}

impl Default for PmxFolderSource {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
        }
    }
}

impl PmxFolderSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    pub fn resolve_case_insensitive(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        if path.is_absolute() {
            return if path.exists() {
                Some(path.to_path_buf())
            } else {
                None
            };
        }

        let mut current = self.root.clone();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !current.pop() {
                        return None;
                    }
                }
                Component::Normal(part) => {
                    let direct = current.join(part);
                    if direct.exists() {
                        current = direct;
                        continue;
                    }

                    let target = part.to_string_lossy().to_lowercase();
                    let mut matched = None;
                    let entries = fs::read_dir(&current).ok()?;
                    for entry in entries {
                        let entry = entry.ok()?;
                        if entry.file_name().to_string_lossy().to_lowercase() == target {
                            matched = Some(entry.path());
                            break;
                        }
                    }
                    current = matched?;
                }
                Component::RootDir | Component::Prefix(_) => {
                    return None;
                }
            }
        }

        Some(current)
    }
}

/// The character set used to decode ZIP entry names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZipNameEncoding {
    Auto,
    Utf8,
    Cp932,
    ShiftJis,
    Gbk,
}

impl Default for ZipNameEncoding {
    fn default() -> Self {
        Self::Auto
    }
}

impl ZipNameEncoding {
    /// Decodes a ZIP entry name using this encoding.
    pub fn decode_name<'a>(self, raw: &'a [u8]) -> Option<Cow<'a, str>> {
        match self {
            Self::Auto => std::str::from_utf8(raw)
                .map(Cow::Borrowed)
                .ok()
                .or_else(|| {
                    SHIFT_JIS
                        .decode_without_bom_handling_and_without_replacement(raw)
                        .or_else(|| GBK.decode_without_bom_handling_and_without_replacement(raw))
                }),
            Self::Utf8 => std::str::from_utf8(raw).map(Cow::Borrowed).ok(),
            Self::Cp932 | Self::ShiftJis => {
                SHIFT_JIS.decode_without_bom_handling_and_without_replacement(raw)
            }
            Self::Gbk => GBK.decode_without_bom_handling_and_without_replacement(raw),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxZipSource {
    pub archive: PathBuf,
    pub root: String,
    pub name_encoding: ZipNameEncoding,
}

impl PmxZipSource {
    pub fn new(archive: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self::with_encoding(archive, root, ZipNameEncoding::Auto)
    }

    pub fn with_encoding(
        archive: impl Into<PathBuf>,
        root: impl Into<PathBuf>,
        name_encoding: ZipNameEncoding,
    ) -> Self {
        let archive = archive.into();
        let root = root.into();
        let root = root.to_string_lossy();
        Self {
            archive,
            root: normalize_zip_entry_lossy(&root),
            name_encoding,
        }
    }

    pub fn archive(&self) -> &Path {
        &self.archive
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn name_encoding(&self) -> ZipNameEncoding {
        self.name_encoding
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PmxSourceLocation {
        PmxSourceLocation::Zip {
            archive: self.archive.clone(),
            entry: join_zip_entry_lossy(&self.root, path.as_ref()),
        }
    }

    pub fn resolve_case_insensitive(&self, path: impl AsRef<Path>) -> Option<PmxSourceLocation> {
        let target = join_zip_entry_strict(&self.root, path.as_ref())?;
        find_zip_entry(&self.archive, &target, self.name_encoding, true)
            .ok()
            .flatten()
            .map(|entry| PmxSourceLocation::Zip {
                archive: self.archive.clone(),
                entry: entry.entry,
            })
    }

    pub fn contains_entry(&self, entry: impl AsRef<str>) -> bool {
        let Some(target) = normalize_zip_entry_name(entry.as_ref()) else {
            return false;
        };

        find_zip_entry(&self.archive, &target, self.name_encoding, false)
            .ok()
            .flatten()
            .is_some()
    }

    pub fn read_bytes(&self, entry: impl AsRef<str>) -> io::Result<Vec<u8>> {
        let Some(target) = normalize_zip_entry_name(entry.as_ref()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "zip entry name could not be normalized",
            ));
        };
        let Some(match_entry) = find_zip_entry(&self.archive, &target, self.name_encoding, false)?
        else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "zip entry '{}' was not found in {}",
                    target,
                    self.archive.display()
                ),
            ));
        };

        read_zip_entry_bytes(&self.archive, match_entry.index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PmxSourceLocation {
    Disk(PathBuf),
    Zip { archive: PathBuf, entry: String },
}

impl PmxSourceLocation {
    pub fn disk(path: impl Into<PathBuf>) -> Self {
        Self::Disk(path.into())
    }

    pub fn zip(archive: impl Into<PathBuf>, entry: impl Into<String>) -> Self {
        Self::Zip {
            archive: archive.into(),
            entry: entry.into(),
        }
    }

    pub fn as_disk_path(&self) -> Option<&Path> {
        match self {
            Self::Disk(path) => Some(path),
            Self::Zip { .. } => None,
        }
    }

    pub fn as_path(&self) -> &Path {
        match self {
            Self::Disk(path) => path.as_path(),
            Self::Zip { entry, .. } => Path::new(entry),
        }
    }

    pub fn archive(&self) -> Option<&Path> {
        match self {
            Self::Disk(_) => None,
            Self::Zip { archive, .. } => Some(archive),
        }
    }

    pub fn entry(&self) -> Option<&str> {
        match self {
            Self::Disk(_) => None,
            Self::Zip { entry, .. } => Some(entry),
        }
    }

    pub fn extension(&self) -> Option<&str> {
        match self {
            Self::Disk(path) => path.extension().and_then(|ext| ext.to_str()),
            Self::Zip { entry, .. } => Path::new(entry).extension().and_then(|ext| ext.to_str()),
        }
    }
}

impl AsRef<Path> for PmxSourceLocation {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<PmxSourceLocation> for PathBuf {
    fn from(value: PmxSourceLocation) -> Self {
        match value {
            PmxSourceLocation::Disk(path) => path,
            PmxSourceLocation::Zip { entry, .. } => PathBuf::from(entry),
        }
    }
}

impl fmt::Display for PmxSourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disk(path) => write!(f, "{}", path.display()),
            Self::Zip { archive, entry } => write!(f, "{}::{}", archive.display(), entry),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PmxSource {
    Folder(PmxFolderSource),
    Zip(PmxZipSource),
}

impl Default for PmxSource {
    fn default() -> Self {
        Self::Folder(PmxFolderSource::default())
    }
}

impl PmxSource {
    pub fn folder(root: impl Into<PathBuf>) -> Self {
        Self::Folder(PmxFolderSource::new(root))
    }

    pub fn zip(archive: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self::Zip(PmxZipSource::new(archive, root))
    }

    pub fn zip_with_encoding(
        archive: impl Into<PathBuf>,
        root: impl Into<PathBuf>,
        name_encoding: ZipNameEncoding,
    ) -> Self {
        Self::Zip(PmxZipSource::with_encoding(archive, root, name_encoding))
    }

    pub fn as_folder(&self) -> Option<&PmxFolderSource> {
        match self {
            Self::Folder(source) => Some(source),
            Self::Zip(_) => None,
        }
    }

    pub fn as_zip(&self) -> Option<&PmxZipSource> {
        match self {
            Self::Folder(_) => None,
            Self::Zip(source) => Some(source),
        }
    }

    pub fn root(&self) -> &Path {
        match self {
            Self::Folder(source) => source.root(),
            Self::Zip(source) => source.archive(),
        }
    }

    pub fn root_path(&self) -> Option<&Path> {
        match self {
            Self::Folder(source) => Some(source.root()),
            Self::Zip(_) => None,
        }
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PmxSourceLocation {
        let path = path.as_ref();
        if path.is_absolute() {
            return PmxSourceLocation::Disk(path.to_path_buf());
        }

        match self {
            Self::Folder(source) => PmxSourceLocation::Disk(source.resolve(path)),
            Self::Zip(source) => source.resolve(path),
        }
    }

    pub fn resolve_case_insensitive(&self, path: impl AsRef<Path>) -> Option<PmxSourceLocation> {
        let path = path.as_ref();
        if path.is_absolute() {
            return if path.exists() {
                Some(PmxSourceLocation::Disk(path.to_path_buf()))
            } else {
                None
            };
        }

        match self {
            Self::Folder(source) => source
                .resolve_case_insensitive(path)
                .map(PmxSourceLocation::Disk),
            Self::Zip(source) => source.resolve_case_insensitive(path),
        }
    }

    pub fn contains_location(&self, location: &PmxSourceLocation) -> bool {
        match location {
            PmxSourceLocation::Disk(path) => path.exists(),
            PmxSourceLocation::Zip { archive, entry } => match self {
                Self::Folder(_) => false,
                Self::Zip(source) => {
                    if source.archive != *archive {
                        return false;
                    }
                    source.contains_entry(entry)
                }
            },
        }
    }

    pub fn read_bytes(&self, location: &PmxSourceLocation) -> io::Result<Vec<u8>> {
        match location {
            PmxSourceLocation::Disk(path) => fs::read(path),
            PmxSourceLocation::Zip { archive, entry } => match self {
                Self::Folder(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "zip entry cannot be read from a folder source",
                )),
                Self::Zip(source) => {
                    if source.archive != *archive {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "zip entry archive does not match the source archive",
                        ));
                    }
                    source.read_bytes(entry)
                }
            },
        }
    }
}

impl From<PmxFolderSource> for PmxSource {
    fn from(source: PmxFolderSource) -> Self {
        Self::Folder(source)
    }
}

impl From<PmxZipSource> for PmxSource {
    fn from(source: PmxZipSource) -> Self {
        Self::Zip(source)
    }
}

fn read_zip_entry_bytes(archive_path: &Path, index: usize) -> io::Result<Vec<u8>> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error_to_io)?;
    let mut entry = archive.by_index(index).map_err(zip_error_to_io)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn find_zip_entry(
    archive_path: &Path,
    target: &str,
    encoding: ZipNameEncoding,
    case_insensitive: bool,
) -> io::Result<Option<ZipEntryMatch>> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(zip_error_to_io)?;

    let target = match normalize_zip_entry_name(target) {
        Some(value) => value,
        None => return Ok(None),
    };
    let target_lower = case_insensitive.then(|| target.to_lowercase());
    let mut case_match = None;

    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).map_err(zip_error_to_io)?;
        if file.is_dir() || is_zip_noise_entry(file.name_raw()) {
            continue;
        }

        for candidate in decode_zip_entry_candidates(&file, encoding) {
            let Some(candidate) = normalize_zip_entry_name(&candidate) else {
                continue;
            };

            if candidate == target {
                return Ok(Some(ZipEntryMatch {
                    index,
                    entry: candidate,
                }));
            }

            if case_insensitive
                && candidate.to_lowercase() == target_lower.as_deref().expect("case target")
            {
                case_match.get_or_insert_with(|| ZipEntryMatch {
                    index,
                    entry: candidate,
                });
            }
        }
    }

    Ok(case_match)
}

pub(crate) fn find_first_pmx_zip_entry_root(
    archive_bytes: &[u8],
    encoding: ZipNameEncoding,
) -> io::Result<Option<(usize, String, String)>> {
    let cursor = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(zip_error_to_io)?;

    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).map_err(zip_error_to_io)?;
        if file.is_dir() || is_zip_noise_entry(file.name_raw()) {
            continue;
        }

        for candidate in decode_zip_entry_candidates(&file, encoding) {
            let Some(candidate) = normalize_zip_entry_name(&candidate) else {
                continue;
            };

            if is_pmx_entry_name(&candidate) {
                let root = Path::new(&candidate)
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                return Ok(Some((index, candidate, root)));
            }
        }
    }

    Ok(None)
}

fn decode_zip_entry_candidates<R: Read + ?Sized>(
    file: &ZipFile<'_, R>,
    encoding: ZipNameEncoding,
) -> Vec<String> {
    let mut candidates = Vec::new();
    match encoding {
        ZipNameEncoding::Auto => {
            if let Ok(name) = file.name() {
                push_unique(&mut candidates, name.into_owned());
            }
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::Utf8);
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::Cp932);
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::ShiftJis);
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::Gbk);
        }
        ZipNameEncoding::Utf8 => {
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::Utf8);
        }
        ZipNameEncoding::Cp932 | ZipNameEncoding::ShiftJis => {
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::ShiftJis);
        }
        ZipNameEncoding::Gbk => {
            push_unique_from_raw(&mut candidates, file.name_raw(), ZipNameEncoding::Gbk);
        }
    }
    candidates
}

fn push_unique(candidates: &mut Vec<String>, candidate: String) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn push_unique_from_raw(candidates: &mut Vec<String>, raw: &[u8], encoding: ZipNameEncoding) {
    let Some(candidate) = decode_zip_name(raw, encoding) else {
        return;
    };
    push_unique(candidates, candidate);
}

fn decode_zip_name(raw: &[u8], encoding: ZipNameEncoding) -> Option<String> {
    match encoding {
        ZipNameEncoding::Auto => unreachable!("Auto is only used for candidate selection"),
        ZipNameEncoding::Utf8 => std::str::from_utf8(raw).ok().map(str::to_owned),
        ZipNameEncoding::Cp932 | ZipNameEncoding::ShiftJis => {
            let (decoded, had_errors) = SHIFT_JIS.decode_without_bom_handling(raw);
            (!had_errors).then(|| decoded.into_owned())
        }
        ZipNameEncoding::Gbk => {
            let (decoded, had_errors) = GBK.decode_without_bom_handling(raw);
            (!had_errors).then(|| decoded.into_owned())
        }
    }
}

fn normalize_zip_entry_name(value: &str) -> Option<String> {
    let mut components = Vec::new();
    let normalized = value.replace('\\', "/");

    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            "__MACOSX" => return None,
            part if part.starts_with("._") => return None,
            ".." => {
                components.pop()?;
            }
            part => components.push(part),
        }
    }

    Some(components.join("/"))
}

fn normalize_zip_entry_lossy(value: &str) -> String {
    normalize_zip_entry_name(value).unwrap_or_else(|| value.replace('\\', "/"))
}

fn join_zip_entry_lossy(root: &str, path: &Path) -> String {
    let path = {
        let path = path.to_string_lossy();
        normalize_zip_entry_lossy(&path)
    };
    if root.is_empty() {
        return path;
    }
    if path.is_empty() {
        return root.to_owned();
    }
    format!("{root}/{path}")
}

fn join_zip_entry_strict(root: &str, path: &Path) -> Option<String> {
    let path = {
        let path = path.to_string_lossy();
        normalize_zip_entry_name(&path)?
    };
    if root.is_empty() {
        return Some(path);
    }
    if path.is_empty() {
        return Some(root.to_owned());
    }
    Some(format!("{root}/{path}"))
}

fn is_zip_noise_entry(name_raw: &[u8]) -> bool {
    let Ok(name) = std::str::from_utf8(name_raw) else {
        return false;
    };
    name.split(['/', '\\'])
        .any(|part| part == "__MACOSX" || part.starts_with("._"))
}

fn is_pmx_entry_name(entry: &str) -> bool {
    Path::new(entry)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pmx"))
}

fn zip_error_to_io(error: zip::result::ZipError) -> io::Error {
    io::Error::other(error)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZipEntryMatch {
    index: usize,
    entry: String,
}

#[cfg(test)]
mod tests {
    use super::{PmxSource, PmxSourceLocation, ZipNameEncoding, normalize_zip_entry_name};
    use encoding_rs::{GBK, SHIFT_JIS};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{unique}{extension}"))
    }

    fn write_u16(buf: &mut Vec<u8>, value: u16) {
        buf.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(buf: &mut Vec<u8>, value: u32) {
        buf.extend_from_slice(&value.to_le_bytes());
    }

    fn write_zip(entries: Vec<&[u8]>) -> PathBuf {
        let zip_path = unique_temp_path("bevy_pmx_source", ".zip");
        let mut file_bytes = Vec::new();
        let mut central_directory = Vec::new();
        let mut local_header_offset = 0u32;
        let entry_count =
            u16::try_from(entries.len()).expect("entry count should fit in ZIP header");

        for name in entries {
            let name_length = u16::try_from(name.len()).expect("name should fit in ZIP header");

            write_u32(&mut file_bytes, 0x0403_4b50);
            write_u16(&mut file_bytes, 20);
            write_u16(&mut file_bytes, 0);
            write_u16(&mut file_bytes, 0);
            write_u16(&mut file_bytes, 0);
            write_u16(&mut file_bytes, 0);
            write_u32(&mut file_bytes, 0);
            write_u32(&mut file_bytes, 0);
            write_u32(&mut file_bytes, 0);
            write_u16(&mut file_bytes, name_length);
            write_u16(&mut file_bytes, 0);
            file_bytes.extend_from_slice(name);

            write_u32(&mut central_directory, 0x0201_4b50);
            write_u16(&mut central_directory, 20);
            write_u16(&mut central_directory, 20);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u32(&mut central_directory, 0);
            write_u32(&mut central_directory, 0);
            write_u32(&mut central_directory, 0);
            write_u16(&mut central_directory, name_length);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u16(&mut central_directory, 0);
            write_u32(&mut central_directory, 0);
            write_u32(&mut central_directory, local_header_offset);
            central_directory.extend_from_slice(name);

            local_header_offset += 30 + u32::from(name_length);
        }

        let central_directory_offset = u32::try_from(file_bytes.len())
            .expect("central directory offset should fit in ZIP header");
        let central_directory_size = u32::try_from(central_directory.len())
            .expect("central directory size should fit in ZIP header");

        file_bytes.extend_from_slice(&central_directory);

        write_u32(&mut file_bytes, 0x0605_4b50);
        write_u16(&mut file_bytes, 0);
        write_u16(&mut file_bytes, 0);
        write_u16(&mut file_bytes, entry_count);
        write_u16(&mut file_bytes, entry_count);
        write_u32(&mut file_bytes, central_directory_size);
        write_u32(&mut file_bytes, central_directory_offset);
        write_u16(&mut file_bytes, 0);

        fs::write(&zip_path, file_bytes).expect("should write zip fixture");
        zip_path
    }

    #[test]
    fn zip_name_normalization_unifies_separators_and_ignores_mac_noise() {
        assert_eq!(
            normalize_zip_entry_name(r"Texture\Face.PNG").as_deref(),
            Some("Texture/Face.PNG")
        );
        assert_eq!(normalize_zip_entry_name("__MACOSX/._Face.PNG"), None);
    }

    #[test]
    fn auto_zip_source_resolves_shift_jis_entry_names() {
        let (name, _, _) = SHIFT_JIS.encode("Texture/あ.png");
        let zip_path = write_zip(vec![name.as_ref()]);
        let source = PmxSource::zip_with_encoding(&zip_path, "", ZipNameEncoding::Auto);

        let resolved = source
            .resolve_case_insensitive("Texture/あ.png")
            .expect("auto should resolve a shift-jis entry");

        assert_eq!(
            resolved,
            PmxSourceLocation::Zip {
                archive: zip_path,
                entry: "Texture/あ.png".to_owned(),
            }
        );
    }

    #[test]
    fn explicit_zip_encoding_does_not_guess_other_encodings() {
        let (name, _, _) = SHIFT_JIS.encode("Texture/あ.png");
        let zip_path = write_zip(vec![name.as_ref()]);
        let source = PmxSource::zip_with_encoding(&zip_path, "", ZipNameEncoding::Gbk);

        assert!(
            source.resolve_case_insensitive("Texture/あ.png").is_none(),
            "gbk should not guess a shift-jis entry"
        );
    }

    #[test]
    fn explicit_zip_encoding_reads_the_specified_charset() {
        let (name, _, _) = GBK.encode("Texture/汉.png");
        let zip_path = write_zip(vec![name.as_ref()]);
        let source = PmxSource::zip_with_encoding(&zip_path, "", ZipNameEncoding::Gbk);

        let resolved = source
            .resolve_case_insensitive("Texture/汉.png")
            .expect("gbk source should resolve a gbk entry");

        assert_eq!(
            resolved,
            PmxSourceLocation::Zip {
                archive: zip_path,
                entry: "Texture/汉.png".to_owned(),
            }
        );
    }
}
