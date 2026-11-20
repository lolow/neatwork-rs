//! End-to-end flow of a NW5 project on the examples of
//! `reference/neatwork5/examples`: import, revisions, archive, graph state,
//! runs on a background thread, reopen, review, comparison, export.

use std::path::{Path, PathBuf};

use neatwork_nw5_project::compare::{
    CompatibilityLevel, compare_run_metrics, compatibility, default_pair, match_strategies,
    metric_table, recommended_pair, unified_table,
};
use neatwork_nw5_project::graph::EdgeKind;
use neatwork_nw5_project::project::RevisionExport;
use neatwork_nw5_project::review::{TableKind, result_tables, run_summary};
use neatwork_nw5_project::run::ParameterSource;
use neatwork_nw5_project::{
    CancelFlag, DisplayUnit, Error, ImportOptions, Project, RequestedOptions, RevisionName,
    RunStatus, SourceFormat,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/neatwork5/examples")
}

/// A fresh folder under the system temporary folder.
fn scratch(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("nw5-project-{name}-{}", std::process::id()));
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

/// Run a job on a background thread, as the interface does.
fn run_in_thread(
    project: &mut Project,
    request: &RequestedOptions,
) -> Result<neatwork_nw5_project::RunRecord, Box<dyn std::error::Error>> {
    let job = project.start_run(request)?;
    let cancel = CancelFlag::new();
    let handle = std::thread::spawn(move || {
        let mut calls = 0usize;
        let r = job.execute(&cancel, |_, _| calls += 1);
        (r, calls)
    });
    let (record, calls) = handle.join().map_err(|_| "run thread panicked")?;
    project.finish_run();
    assert!(calls > 0);
    Ok(record?)
}

#[test]
fn dsg_project_full_flow() -> TestResult {
    let parent = scratch("dsg")?;
    let mut p = Project::create(&parent, "  Toy network ")?;
    assert_eq!(p.name(), "Toy network");
    assert_eq!(p.source_format(), SourceFormat::None);
    assert_eq!(p.display_unit(), DisplayUnit::Meter);
    assert!(p.root().join("inputs").is_dir() && p.root().join("runs").is_dir());

    // import a .dsg: copied as <base>_v1.dsg, sets the project format
    let toy = examples().join("dsg_examples/Toy/Toy_Secu1.00_fusion0_split0.dsg");
    let imp = p.import_network(&toy, &ImportOptions::default())?;
    let v1 = imp.revision.file.clone();
    assert_eq!(v1, "Toy_Secu1.00_fusion0_split0_v1.dsg");
    assert_eq!(p.source_format(), SourceFormat::Dsg);
    assert_eq!(
        std::fs::read(p.root().join("inputs").join(&v1))?,
        std::fs::read(&toy)?
    );
    // the same file again is reused; a .tpo is refused by the format lock
    assert!(p.import_network(&toy, &ImportOptions::default())?.reused);
    let ceiba = examples().join("tpo_examples/Ceiba/Ceiba.tpo");
    assert_eq!(
        refused(p.import_network(&ceiba, &ImportOptions::default())),
        "This project uses DSG input. A TPO file cannot be added to it."
    );

    // graph: types, generated layout (all coordinates are 0), user state
    let g = p.graph()?;
    assert_eq!(g.nodes.len(), 8);
    assert!(
        g.nodes
            .iter()
            .all(|n| n.layout_generated || (n.x == 0.0 && n.y == 0.0))
    );
    assert!(g.edges.iter().all(|e| e.kind == EdgeKind::Pipe && e.open));
    assert_eq!(
        g.find_node("p3").map(|n| n.id.clone()),
        Ok("P3".to_string())
    );
    p.set_node_position("A", 10.0, 20.0)?;
    p.set_arc_open("A7", false)?;
    assert!(refused(p.set_node_position("Nope", 1.0, 1.0)).starts_with("Node not found"));
    assert!(refused(p.set_arc_open("A99", false)).starts_with("Edge not found"));

    // two revisions; a duplicate name is refused; unknown sections kept
    let mut net = p.load_active()?.network;
    net.params.set("pressure", "3");
    net.extra_sections
        .push(("!Custom".into(), vec!["kept,line".into()]));
    let v2 = p.save_revision(&net, &RevisionName::Automatic)?;
    assert_eq!(v2.file, "Toy_Secu1.00_fusion0_split0_v2.dsg");
    assert_eq!(v2.version, 2);
    assert_eq!(v2.parent.as_deref(), Some(v1.as_str()));
    assert_eq!(
        p.active_revision().map(|r| r.file.clone()),
        Some(v2.file.clone())
    );
    let text = p.revision_text(&v2.file)?;
    assert!(text.contains("!Custom\nkept,line\n") && text.contains("!loadfactor"));
    assert_eq!(p.load_active()?.network.params.raw("pressure"), Some("3"));
    let dup = RevisionName::Custom(v2.file.clone());
    assert!(refused(p.save_revision(&net, &dup)).starts_with("A revision named"));
    let mine = p.save_revision(&net, &RevisionName::Custom(" Toy_mine ".into()))?;
    assert_eq!((mine.file.as_str(), mine.version), ("Toy_mine.dsg", 3));
    // the user state survives the new revisions (D11)
    let g = p.graph()?;
    let a = g.nodes.iter().find(|n| n.id == "A").ok_or("A")?;
    assert_eq!((a.x, a.y), (10.0, 20.0));
    assert!(!g.edges.iter().find(|e| e.id == "A7").ok_or("A7")?.open);

    // archive: kept, readable for runs, not editable
    p.archive_revision(&mine.file)?;
    assert!(p.revision(&mine.file).is_some_and(|r| r.archived));
    assert!(refused(p.save_revision(&net, &RevisionName::Automatic)).contains("archived"));
    assert!(p.revision_text(&mine.file).is_err());
    assert!(p.load_revision(&mine.file).is_ok());

    // run 1 on v2 (seed, small count), on a background thread
    p.set_active_revision(&v2.file)?;
    let request = RequestedOptions {
        scenario_count: Some(20),
        seed: Some(7),
        ..RequestedOptions::default()
    };
    let r1 = run_in_thread(&mut p, &request)?;
    assert_eq!(r1.run_number, 1);
    assert!(r1.status.is_completed(), "{:?} {:?}", r1.status, r1.error);
    assert_eq!(r1.revision, v2.file);
    assert_eq!(r1.effective_seed(), Some(7));
    assert_eq!(r1.scenario_count(), Some(20));
    assert_eq!(
        r1.resolved_value("seed").map(|v| v.source),
        Some(ParameterSource::UserOverride)
    );
    assert_eq!(
        r1.resolved_value("uniformDemand").map(|v| v.source),
        Some(ParameterSource::ClientDemandTable)
    );
    assert!(r1.edge_states.contains(&("A7".to_string(), false)));
    assert!(r1.statistics.is_some() && r1.started_at.is_some() && r1.completed_at.is_some());
    assert_eq!(p.active_run(), None);

    // run 2 on the older revision v1, chosen as active (ADR-025 item 5)
    p.set_active_revision(&v1)?;
    let r2 = run_in_thread(&mut p, &request)?;
    assert_eq!((r2.run_number, r2.revision.as_str()), (2, v1.as_str()));
    assert!(r2.status.is_completed());

    // a cancelled run, and one run at a time
    let job = p.start_run(&request)?;
    assert!(refused(p.start_run(&request)).contains("already running"));
    assert!(refused(p.set_arc_open("A1", true)).starts_with("Cancel the running simulation"));
    let cancel = CancelFlag::new();
    cancel.cancel();
    let r3 = job.execute(&cancel, |_, _| {})?;
    p.finish_run();
    assert_eq!(r3.status, RunStatus::Cancelled);

    p.set_display_unit(DisplayUnit::Feet)?;
    p.set_comparison_workspace(1, 2, "pressure")?;

    // reopen: same state (D1)
    let q = Project::open(p.root())?;
    assert_eq!(q.meta(), p.meta());
    assert_eq!(q.revisions(), p.revisions());
    assert_eq!(q.graph_state(&v1), p.graph_state(&v1));
    let runs = q.runs()?;
    assert_eq!(runs, vec![r1.clone(), r2.clone(), r3.clone()]);
    assert_eq!(q.display_unit(), DisplayUnit::Feet);
    assert_eq!(
        q.comparison_workspace().map(|c| (c.left, c.right)),
        Some((1, 2))
    );
    assert_eq!(
        q.active_revision().map(|r| r.file.clone()),
        Some(v1.clone())
    );

    // review from the stored record
    let tables = result_tables(&runs[0], DisplayUnit::Feet);
    assert_eq!(tables.len(), 4);
    assert_eq!(tables[0].rows.len(), 4);
    assert_eq!(tables[3].columns[2].header(), "Minimum (ft)");
    let s = run_summary(&runs[0]);
    assert_eq!(s.strip[1], ("Scenarios", "20".to_string()));
    assert_eq!(s.configuration, "Manual");

    // comparison of the runs of two revisions
    assert_eq!(default_pair(&runs, None), Some((1, 2)));
    let (ln, rn) = (
        q.load_revision(&runs[0].revision)?,
        q.load_revision(&runs[1].revision)?,
    );
    let c = compatibility(
        &ln.network,
        &runs[0].revision,
        &rn.network,
        &runs[1].revision,
    );
    assert_eq!(c.level, CompatibilityLevel::SameTopology);
    assert_eq!(c.pipes.matched.len(), 7);
    let same = compatibility(
        &ln.network,
        &runs[0].revision,
        &ln.network,
        &runs[0].revision,
    );
    assert_eq!(same.level, CompatibilityLevel::SameSnapshot);
    let t = unified_table(
        &runs[0],
        &runs[1],
        TableKind::PipeVelocity,
        Some(&c),
        DisplayUnit::Meter,
    );
    assert!(t.matched > 0 && t.left_only == 0 && t.right_only == 0);
    let t = unified_table(
        &runs[0],
        &runs[1],
        TableKind::Pressure,
        Some(&c),
        DisplayUnit::Meter,
    );
    assert_eq!(t.matched, 7);
    let (lk, rk) = recommended_pair(TableKind::Pressure, TableKind::Pressure).ok_or("pair")?;
    let strategy = &match_strategies(TableKind::Pressure, TableKind::Pressure)[0];
    let m = compare_run_metrics(&runs[0], &runs[1], TableKind::Pressure, lk, rk, strategy)
        .ok_or("metric")?;
    assert_eq!(m.rows.len(), 7);
    let mt = metric_table(&m, DisplayUnit::Meter);
    assert!(mt.rows.iter().all(|r| !r.0[5].ends_with("%%")));

    // export
    let st = q.export_statistics(1)?;
    assert!(st.path.ends_with("exports/simulation_statistics_v1.json"));
    assert!(std::fs::read_to_string(&st.path)?.contains("\"terminalSatisfaction\""));
    assert!(
        q.export_statistics(1)?
            .path
            .ends_with("simulation_statistics_v2.json")
    );
    assert!(refused(q.export_statistics(3)).contains("result tables"));
    for (fmt, ext) in [
        (RevisionExport::Tpo, "tpo"),
        (RevisionExport::Dsg, "dsg"),
        (RevisionExport::NwJson, "json"),
        (RevisionExport::Inp, "inp"),
    ] {
        let e = q.export_revision(&v2.file, fmt)?;
        assert_eq!(e.path.extension().and_then(|x| x.to_str()), Some(ext));
        assert!(std::fs::metadata(&e.path)?.len() > 0);
    }
    let tpo = q.export_revision(&v1, RevisionExport::Tpo)?;
    assert_eq!(
        neatwork_nw5::read::read_file(&tpo.path)?
            .network
            .nodes
            .len(),
        8
    );

    // delete a run, list and delete the project
    let mut q = q;
    q.delete_run(2)?;
    assert_eq!(q.runs()?.len(), 2);
    assert!(q.comparison_workspace().is_none());
    assert!(
        Project::list(&parent)
            .iter()
            .any(|e| e.name == "Toy network")
    );
    let root = q.root().to_path_buf();
    q.delete()?;
    assert!(!root.exists());
    std::fs::remove_dir_all(&parent)?;
    Ok(())
}

#[test]
fn tpo_import_copies_its_catalogues() -> TestResult {
    let parent = scratch("tpo")?;
    let mut p = Project::create(&parent, "Ceiba")?;
    // a .tpo without !Diameters and no catalogue next to it is refused
    let alone = parent.join("alone");
    std::fs::create_dir_all(&alone)?;
    std::fs::copy(
        examples().join("tpo_examples/Ceiba/Ceiba.tpo"),
        alone.join("Ceiba.tpo"),
    )?;
    assert!(
        refused(p.import_network(&alone.join("Ceiba.tpo"), &ImportOptions::default()))
            .starts_with("TPO import requires a diameters database")
    );
    assert_eq!(p.source_format(), SourceFormat::None);

    let ceiba = examples().join("tpo_examples/Ceiba/Ceiba.tpo");
    let imp = p.import_network(&ceiba, &ImportOptions::default())?;
    let r = imp.revision;
    assert_eq!(r.file, "Ceiba_v1.tpo");
    assert_eq!(r.diameters_db.as_deref(), Some("DiamBaseCeiba.db"));
    assert_eq!(r.orifices_db.as_deref(), Some("orifices.db"));
    assert!(p.root().join("db/DiamBaseCeiba.db").is_file());
    assert!(p.root().join("db/orifices.db").is_file());
    let loaded = p.load_active()?;
    assert!(!loaded.network.pipes.is_empty() && !loaded.network.orifices.is_empty());
    let g = p.graph()?;
    assert!(g.edges.iter().all(|e| e.length_m >= 0.01));
    assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Pipe));

    // a revision keeps the link to the unchanged catalogues
    let v2 = p.save_revision(&loaded.network, &RevisionName::Automatic)?;
    assert_eq!(v2.file, "Ceiba_v2.tpo");
    assert_eq!(v2.diameters_db.as_deref(), Some("DiamBaseCeiba.db"));
    assert!(!p.revision_text(&v2.file)?.contains("!Diameters"));
    assert_eq!(p.load_active()?.network.pipes, loaded.network.pipes);

    // a topography cannot be simulated: the run is stored as FAILED
    let job = p.start_run(&RequestedOptions {
        scenario_count: Some(5),
        seed: Some(1),
        ..RequestedOptions::default()
    })?;
    let rec = job.execute(&CancelFlag::new(), |_, _| {})?;
    p.finish_run();
    assert_eq!(rec.status, RunStatus::Failed);
    assert!(rec.error.is_some() && rec.statistics.is_none() && rec.resolved.is_empty());

    let q = Project::open(p.root())?;
    assert_eq!(q.revisions(), p.revisions());
    assert_eq!(q.run(1)?, rec);
    q.delete()?;
    std::fs::remove_dir_all(&parent)?;
    Ok(())
}
