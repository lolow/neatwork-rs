//! A NW5 design saved as a `.dsg` revision of a `.tpo` project, a run of
//! 200 scenarios on it, and the import of converted files (`.json`,
//! `.inp`). The interface uses the same calls.

use std::path::{Path, PathBuf};

use neatwork_nw5_design::{DesignOptions, design};
use neatwork_nw5_project::{
    CancelFlag, Error, ImportOptions, Project, RequestedOptions, RevisionName, RunStatus,
    SourceFormat,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/neatwork5/examples")
}

fn scratch(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("nw5-design-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn refused(r: Result<impl std::fmt::Debug, Error>) -> String {
    match r {
        Err(Error::Refused(m)) => m,
        other => format!("not refused: {other:?}"),
    }
}

/// Ceiba: import, design with the default options, save as revision 2,
/// run 200 scenarios with seed 1. The statistics are written to
/// `<temp>/nw5-ceiba-v2-200-seed1.json`, to compare with an export of the
/// interface.
#[test]
fn ceiba_design_revision_and_run() -> TestResult {
    let parent = scratch("ceiba")?;
    let mut p = Project::create(&parent, "Ceiba")?;
    let ceiba = examples().join("tpo_examples/Ceiba/Ceiba.tpo");
    p.import_network(&ceiba, &ImportOptions::default())?;
    let tpo = p.load_active()?;

    let made = design(&tpo.network, &DesignOptions::default())?;
    assert_eq!(
        p.design_revision_name(&RevisionName::Automatic)?,
        "Ceiba_v2.dsg"
    );
    assert!(
        refused(p.design_revision_name(&RevisionName::Custom("x.tpo".into())))
            .contains("must use .dsg")
    );
    let v2 = p.save_design_revision(&made.network, &RevisionName::Automatic)?;
    assert_eq!(
        (v2.file.as_str(), v2.version, v2.format),
        ("Ceiba_v2.dsg", 2, SourceFormat::Dsg)
    );
    assert_eq!(v2.parent.as_deref(), Some("Ceiba_v1.tpo"));
    assert_eq!(v2.family, "Ceiba_v1.tpo");
    assert_eq!(p.source_format(), SourceFormat::Tpo);
    assert_eq!(
        p.active_revision().map(|r| r.file.clone()),
        Some(v2.file.clone())
    );
    assert!(p.load_active()?.network.is_design());
    // a taken name is refused; the editor keeps the project format
    p.set_active_revision("Ceiba_v1.tpo")?;
    assert!(
        refused(p.save_design_revision(&made.network, &RevisionName::Custom("Ceiba_v2".into())))
            .starts_with("A revision named Ceiba_v2.dsg already exists")
    );
    p.set_active_revision(&v2.file)?;
    assert_eq!(
        refused(p.save_revision(&made.network, &RevisionName::Automatic)),
        "Revision format does not match project format."
    );

    let job = p.start_run(&RequestedOptions {
        scenario_count: Some(200),
        seed: Some(1),
        ..RequestedOptions::default()
    })?;
    let rec = job.execute(&CancelFlag::new(), |_, _| {})?;
    p.finish_run();
    assert_eq!(rec.status, RunStatus::Completed, "{:?}", rec.error);
    assert_eq!(rec.revision, "Ceiba_v2.dsg");
    let out = p.export_statistics(rec.run_number)?;
    std::fs::copy(
        &out.path,
        std::env::temp_dir().join("nw5-ceiba-v2-200-seed1.json"),
    )?;

    let q = Project::open(p.root())?;
    assert_eq!(q.revisions(), p.revisions());
    q.delete()?;
    std::fs::remove_dir_all(&parent)?;
    Ok(())
}

/// NW-json, application JSON and EPANET files are converted on import.
#[test]
fn converted_imports() -> TestResult {
    let parent = scratch("convert")?;
    let toy = examples().join("dsg_examples/Toy/Toy_Secu1.00_fusion0_split0.dsg");
    let net = neatwork_nw5::read::read_file(&toy)?.network;
    let files = [
        ("Toy.json", neatwork_nw5_io::write_nw_json(&net)),
        ("ToyDomain.json", neatwork_nw5_io::write_domain_json(&net)),
        ("Toy.inp", neatwork_nw5_io::write_inp(&net)?.text),
    ];
    for (name, text) in files {
        let path = parent.join(name);
        std::fs::write(&path, text)?;
        let mut p = Project::create(&parent, name)?;
        let imp = p.import_network(&path, &ImportOptions::default())?;
        let stem = name.split('.').next().unwrap_or(name);
        assert_eq!(imp.revision.file, format!("{stem}_v1.dsg"), "{name}");
        assert_eq!(p.source_format(), SourceFormat::Dsg);
        let loaded = p.load_active()?;
        // the EPANET reader adds a node per reservoir and tank link
        assert!(loaded.network.nodes.len() >= net.nodes.len(), "{name}");
        assert!(loaded.network.arcs.len() >= net.arcs.len(), "{name}");
        p.delete()?;
    }
    let mut p = Project::create(&parent, "Other")?;
    let bad = parent.join("bad.txt");
    std::fs::write(&bad, "x")?;
    assert_eq!(
        refused(p.import_network(&bad, &ImportOptions::default())),
        "Unsupported network file. Drop a .dsg or .tpo file."
    );
    p.import_network(&toy, &ImportOptions::default())?;
    std::fs::remove_dir_all(&parent)?;
    Ok(())
}
