//! Revisions: immutable `.dsg`/`.tpo` files in `inputs/`, their numbering
//! and naming (`specs/app.md` F8–F11) and the `inputs/revisions.json` index.

use crate::json::{Json, get_array, get_bool, get_str, get_u64, obj, opt_s, s};
use crate::{Error, Result};

/// Source format of a project or a revision (F7). `None` is the format of
/// a new project.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceFormat {
    /// No network imported yet.
    #[default]
    None,
    /// Design file `.dsg`.
    Dsg,
    /// Topography file `.tpo`.
    Tpo,
}

impl SourceFormat {
    /// Stored text: `NONE`, `DSG`, `TPO`.
    pub fn as_str(self) -> &'static str {
        match self {
            SourceFormat::None => "NONE",
            SourceFormat::Dsg => "DSG",
            SourceFormat::Tpo => "TPO",
        }
    }

    /// Parse the stored text.
    pub fn parse(text: &str) -> Option<SourceFormat> {
        match text.trim().to_ascii_uppercase().as_str() {
            "NONE" => Some(SourceFormat::None),
            "DSG" => Some(SourceFormat::Dsg),
            "TPO" => Some(SourceFormat::Tpo),
            _ => None,
        }
    }

    /// File extension with the dot (`.dsg`, `.tpo`; empty for `None`).
    pub fn extension(self) -> &'static str {
        match self {
            SourceFormat::None => "",
            SourceFormat::Dsg => ".dsg",
            SourceFormat::Tpo => ".tpo",
        }
    }

    /// Format of a file name, from its extension only (case-insensitive).
    pub fn of_file(name: &str) -> Option<SourceFormat> {
        let (_, ext) = split_name(name);
        match ext.to_ascii_lowercase().as_str() {
            ".dsg" => Some(SourceFormat::Dsg),
            ".tpo" => Some(SourceFormat::Tpo),
            _ => None,
        }
    }
}

/// One input file of a project (`revisions.json` entry).
#[derive(Clone, Debug, PartialEq)]
pub struct Revision {
    /// File name in `inputs/`; also the visible name.
    pub file: String,
    /// Version number (F8).
    pub version: u32,
    /// Revision family: file name of the imported file the family starts
    /// from.
    pub family: String,
    /// File this revision was saved from (`None` for an import).
    pub parent: Option<String>,
    /// `DSG` or `TPO`.
    pub format: SourceFormat,
    /// Creation time (ISO 8601 UTC).
    pub created_at: String,
    /// Archived (F11): kept on disk, hidden from new imports, not editable.
    pub archived: bool,
    /// Diameter catalogue in `db/` used by a `.tpo` without `!Diameters`.
    pub diameters_db: Option<String>,
    /// Orifice catalogue in `db/` used by a `.tpo` without `!Orifices`.
    pub orifices_db: Option<String>,
    /// Path of the file the import copied (imports only).
    pub imported_from: Option<String>,
}

impl Revision {
    /// Network name of the revision: the file name without extension.
    pub fn network_name(&self) -> String {
        split_name(&self.file).0.to_string()
    }

    /// JSON entry.
    pub fn to_json(&self) -> Json {
        obj(vec![
            ("file", s(&self.file)),
            ("version", Json::Number(f64::from(self.version))),
            ("family", s(&self.family)),
            ("parent", opt_s(self.parent.as_deref())),
            ("format", s(self.format.as_str())),
            ("createdAt", s(&self.created_at)),
            ("archived", Json::Bool(self.archived)),
            ("diametersDb", opt_s(self.diameters_db.as_deref())),
            ("orificesDb", opt_s(self.orifices_db.as_deref())),
            ("importedFrom", opt_s(self.imported_from.as_deref())),
        ])
    }

    /// Read a JSON entry.
    pub fn from_json(v: &Json) -> Result<Revision> {
        let file =
            get_str(v, "file").ok_or_else(|| Error::Format("revision without file".into()))?;
        let format = get_str(v, "format")
            .and_then(|f| SourceFormat::parse(&f))
            .or_else(|| SourceFormat::of_file(&file))
            .unwrap_or_default();
        Ok(Revision {
            version: get_u64(v, "version")
                .and_then(|x| u32::try_from(x).ok())
                .unwrap_or(1),
            family: get_str(v, "family").unwrap_or_else(|| file.clone()),
            parent: get_str(v, "parent"),
            format,
            created_at: get_str(v, "createdAt").unwrap_or_default(),
            archived: get_bool(v, "archived").unwrap_or(false),
            diameters_db: get_str(v, "diametersDb"),
            orifices_db: get_str(v, "orificesDb"),
            imported_from: get_str(v, "importedFrom"),
            file,
        })
    }
}

/// Text of `revisions.json`.
pub fn revisions_to_json(revisions: &[Revision]) -> String {
    obj(vec![(
        "revisions",
        Json::Array(revisions.iter().map(Revision::to_json).collect()),
    )])
    .pretty()
}

/// Read `revisions.json`.
pub fn revisions_from_json(text: &str) -> Result<Vec<Revision>> {
    let v = Json::parse(text).map_err(|e| Error::Format(format!("revisions.json: {e}")))?;
    get_array(&v, "revisions")
        .iter()
        .map(Revision::from_json)
        .collect()
}

/// How a saved revision is named (F9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevisionName {
    /// "Save automatically": `<base>_v<next>.<ext>`.
    Automatic,
    /// "Choose file name": a name checked by [`validate_custom_name`].
    Custom(String),
}

/// Base and extension (with the dot) of a file name. A name without a dot
/// has an empty extension.
pub fn split_name(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) => (&name[..dot], &name[dot..]),
        None => (name, ""),
    }
}

/// Position of a trailing `_v<N>` (case-insensitive) in a base name and
/// its number.
fn version_suffix(base: &str) -> Option<(usize, &str)> {
    let digits_start = base.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits_start == base.len() || digits_start < 2 {
        return None;
    }
    let marker = &base[digits_start - 2..digits_start];
    if marker.eq_ignore_ascii_case("_v") {
        Some((digits_start - 2, &base[digits_start..]))
    } else {
        None
    }
}

/// Base name without a trailing `_v<N>`.
pub fn strip_version(base: &str) -> &str {
    version_suffix(base).map_or(base, |(at, _)| &base[..at])
}

/// Version of an imported network file: `N` for a name ending in `_v<N>`
/// before the extension, else 1 (`ProjectArtifactService.java:529-542`).
pub fn initial_version(file: &str) -> u32 {
    let (base, _) = split_name(file);
    version_suffix(base)
        .and_then(|(_, n)| n.parse::<u32>().ok())
        .unwrap_or(1)
}

/// Name of an imported file in `inputs/`: `<base>_v<N><ext>` with `N` from
/// [`initial_version`] and the extension in lower case (`Toy.dsg` →
/// `Toy_v1.dsg`, `Net_v3.DSG` → `Net_v3.dsg`).
// PROPOSAL: spec 14.2 shows `inputs/<base>_v1.dsg` and says the visible
// name is the file name; the original name stays in `importedFrom`.
pub fn import_file_name(file: &str) -> String {
    let (base, ext) = split_name(file.trim());
    format!(
        "{}_v{}{}",
        strip_version(base),
        initial_version(file.trim()),
        ext.to_ascii_lowercase()
    )
}

/// Default name of a new revision: source base without a trailing `_v<N>`,
/// then `_v<version>` and the extension (`Bihajua.dsg` → `Bihajua_v2.dsg`).
pub fn default_revision_name(source: &str, version: u32, extension: &str) -> String {
    let (base, _) = split_name(source.trim());
    format!("{}_v{version}{extension}", strip_version(base))
}

/// Suggested custom name (F10): source base without `_v<N>`, `_edited` and
/// the extension (`Net_v3.dsg` → `Net_edited.dsg`).
pub fn suggested_custom_name(source: &str) -> String {
    let (base, ext) = split_name(source.trim());
    format!("{}_edited{ext}", strip_version(base))
}

/// Check a custom revision name (F8): trimmed, no `/` or `\`, not `.` or
/// `..`, the extension added when the name has no dot, it must end with
/// the source extension, at most 240 characters.
// PROPOSAL: NW5 also applies Unicode NFKC normalization; the port has no
// Unicode normalization table (no new dependency) and keeps the name as
// typed after trimming.
pub fn validate_custom_name(requested: &str, extension: &str) -> Result<String> {
    let mut name = requested.trim().to_string();
    if name.is_empty() {
        return Err(Error::Refused("A custom filename is required.".into()));
    }
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(Error::Refused("Invalid filename.".into()));
    }
    if !name.contains('.') {
        name.push_str(extension);
    }
    if !name.to_lowercase().ends_with(extension) {
        return Err(Error::Refused(format!("The filename must use {extension}")));
    }
    if name.chars().count() > 240 {
        return Err(Error::Refused("Filename is too long.".into()));
    }
    Ok(name)
}

/// Next version number of a family: highest version + 1.
pub fn next_version(revisions: &[Revision], family: &str) -> u32 {
    revisions
        .iter()
        .filter(|r| r.family == family)
        .map(|r| r.version)
        .max()
        .unwrap_or(0)
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_and_names() {
        assert_eq!(initial_version("Toy.dsg"), 1);
        assert_eq!(initial_version("Net_v3.dsg"), 3);
        assert_eq!(initial_version("Net_V12.tpo"), 12);
        assert_eq!(initial_version("Net_v.dsg"), 1);
        assert_eq!(initial_version("v3.dsg"), 1);
        assert_eq!(import_file_name("Toy.DSG"), "Toy_v1.dsg");
        assert_eq!(import_file_name("Net_v3.dsg"), "Net_v3.dsg");
        assert_eq!(
            default_revision_name("Bihajua.dsg", 2, ".dsg"),
            "Bihajua_v2.dsg"
        );
        assert_eq!(default_revision_name("Toy_v1.dsg", 2, ".dsg"), "Toy_v2.dsg");
        assert_eq!(suggested_custom_name("Net_v3.dsg"), "Net_edited.dsg");
    }

    #[test]
    fn custom_names() {
        assert_eq!(
            validate_custom_name("  Mine ", ".dsg"),
            Ok("Mine.dsg".into())
        );
        assert_eq!(
            validate_custom_name("Mine.DSG", ".dsg"),
            Ok("Mine.DSG".into())
        );
        assert!(validate_custom_name("", ".dsg").is_err());
        assert!(validate_custom_name("a/b", ".dsg").is_err());
        assert!(validate_custom_name("..", ".dsg").is_err());
        assert_eq!(
            validate_custom_name("Mine.tpo", ".dsg"),
            Err(Error::Refused("The filename must use .dsg".into()))
        );
        assert!(validate_custom_name(&"x".repeat(240), ".dsg").is_err());
    }

    #[test]
    fn index_round_trip() -> Result<()> {
        let r = Revision {
            file: "Ceiba_v1.tpo".into(),
            version: 1,
            family: "Ceiba_v1.tpo".into(),
            parent: None,
            format: SourceFormat::Tpo,
            created_at: "2026-11-20T00:00:00.000Z".into(),
            archived: true,
            diameters_db: Some("DiamBaseCeiba.db".into()),
            orifices_db: None,
            imported_from: Some("/x/Ceiba.tpo".into()),
        };
        let back = revisions_from_json(&revisions_to_json(std::slice::from_ref(&r)))?;
        assert_eq!(back, vec![r]);
        assert_eq!(next_version(&back, "Ceiba_v1.tpo"), 2);
        assert_eq!(next_version(&back, "Other"), 1);
        Ok(())
    }
}
