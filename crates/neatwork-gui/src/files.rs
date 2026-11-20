//! Project files on disk (Java `FileManagerDisk`).
//!
//! Projects are in `<base>/projects`: `<name>.tpo`, `<name>.dsg`, and the
//! simulations of a design `<design>.<n>.sim`. The catalogs are in
//! `<base>/db`. The Java program uses the working folder as `<base>`.

use std::path::{Path, PathBuf};

use neatwork_core::java::project_file_text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Topo,
    Design,
    Simulation,
}

impl FileKind {
    pub fn suffix(self) -> &'static str {
        match self {
            FileKind::Topo => ".tpo",
            FileKind::Design => ".dsg",
            FileKind::Simulation => ".sim",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Files {
    pub project_dir: PathBuf,
    pub db_dir: PathBuf,
}

impl Files {
    pub fn new(base: &Path) -> Self {
        Files {
            project_dir: base.join("projects"),
            db_dir: base.join("db"),
        }
    }

    fn path(&self, name: &str, kind: FileKind) -> PathBuf {
        self.project_dir.join(format!("{name}{}", kind.suffix()))
    }

    /// Names of the project files of a kind, without the extension,
    /// sorted.
    pub fn list(&self, kind: FileKind) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(&self.project_dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter_map(|f| f.strip_suffix(kind.suffix()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    /// File text; empty when the file does not exist.
    pub fn read(&self, name: &str, kind: FileKind) -> String {
        std::fs::read_to_string(self.path(name, kind))
            .map(|t| t.lines().map(|l| l.to_string() + "\n").collect())
            .unwrap_or_default()
    }

    /// Write a project file; empty lines are dropped, as in Java.
    pub fn write(&self, name: &str, content: &str, kind: FileKind) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.project_dir)?;
        std::fs::write(self.path(name, kind), project_file_text(content))
    }

    /// Delete a project file. A design takes its simulations with it.
    pub fn delete(&self, name: &str, kind: FileKind) -> std::io::Result<()> {
        if kind == FileKind::Design {
            for n in self.simulations(name) {
                let _ =
                    std::fs::remove_file(self.path(&format!("{name}.{n}"), FileKind::Simulation));
            }
        }
        std::fs::remove_file(self.path(name, kind))
    }

    /// Numbers of the simulations of a design (`<design>.<n>.sim`),
    /// sorted.
    pub fn simulations(&self, design: &str) -> Vec<u64> {
        let prefix = format!("{design}.");
        let mut v: Vec<u64> = self
            .list(FileKind::Simulation)
            .iter()
            .filter_map(|f| f.strip_prefix(&prefix).and_then(|n| n.parse().ok()))
            .collect();
        v.sort_unstable();
        v
    }

    /// Catalog file text, empty when it does not exist.
    pub fn read_db(&self, file: &str) -> String {
        std::fs::read_to_string(self.db_dir.join(file)).unwrap_or_default()
    }

    pub fn write_db(&self, file: &str, content: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.db_dir)?;
        std::fs::write(self.db_dir.join(file), content)
    }
}

/// `appli.locale` of the file `neatwork.locale` in the working folder.
pub fn read_locale() -> Option<String> {
    let text = std::fs::read_to_string("neatwork.locale").ok()?;
    let mut p = neatwork_core::java::JProperties::new();
    p.load(&text);
    p.get_property("appli.locale")
        .filter(|v| *v != "null")
        .map(str::to_string)
}

/// Write `neatwork.locale` as `LocaleAction` does.
pub fn write_locale(code: &str) -> std::io::Result<()> {
    std::fs::write(
        "neatwork.locale",
        format!("#Do not touch this file\nappli.locale={code}\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_list_read_delete() -> std::io::Result<()> {
        let dir = std::env::temp_dir().join(format!("nw-files-{}", std::process::id()));
        let f = Files::new(&dir);
        f.write("toy", "!Nodes\n\nA,1\n", FileKind::Design)?;
        f.write("toy.0", "x\n", FileKind::Simulation)?;
        f.write("toy.3", "x\n", FileKind::Simulation)?;
        assert_eq!(f.list(FileKind::Design), vec!["toy"]);
        assert_eq!(f.read("toy", FileKind::Design), "!Nodes\nA,1\n");
        assert_eq!(f.simulations("toy"), vec![0, 3]);
        f.delete("toy", FileKind::Design)?;
        assert!(f.list(FileKind::Simulation).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
