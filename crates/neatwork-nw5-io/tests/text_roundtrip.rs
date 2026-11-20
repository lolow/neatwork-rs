//! `.tpo`/`.dsg`/`.db` writers: round-trip of every example and golden
//! snippets (`specs/formats.md` section 12.3).

mod common;

use common::{Skip, TestResult, compare, example_files, read, reference};
use neatwork_nw5::read::{read_diameters_db, read_dsg, read_orifices_db, read_tpo};
use neatwork_nw5_io::{write_diameters_db, write_dsg, write_orifices_db, write_tpo};

#[test]
fn every_example_round_trips_in_its_own_format() -> TestResult {
    let mut failures = Vec::new();
    let files = example_files()?;
    assert!(files.len() >= 30, "examples not found");
    for p in &files {
        let a = read(p)?;
        let is_dsg = p.extension().is_some_and(|e| e == "dsg");
        let text = if is_dsg { write_dsg(&a) } else { write_tpo(&a) };
        assert!(
            text.ends_with('\n') && !text.contains('\r'),
            "{}: LF and final newline",
            p.display()
        );
        let b = if is_dsg {
            read_dsg(&a.name, &text)?
        } else {
            read_tpo(&a.name, &text)?
        }
        .network;
        let d = compare(&a, &b, Skip::default());
        if !d.is_empty() {
            failures.push(format!("{}:\n  {}", p.display(), d.join("\n  ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn every_dsg_writes_back_to_the_same_text_after_normalisation() -> TestResult {
    // A design written twice gives the same text (the writer is a fixed point).
    for p in example_files()? {
        let a = read(&p)?;
        let t1 = write_dsg(&a);
        let t2 = write_dsg(&read_dsg(&a.name, &t1)?.network);
        assert_eq!(t1, t2, "{}", p.display());
        let t1 = write_tpo(&a);
        let t2 = write_tpo(&read_tpo(&a.name, &t1)?.network);
        assert_eq!(t1, t2, "{}", p.display());
    }
    Ok(())
}

/// The current MATLAB layout of `Toy_bbd_Secu1.00_fusion0_split0.dsg` comes
/// back unchanged, except `ID2 = 0` which becomes `fictif` (12.3 rule 3).
#[test]
fn toy_bbd_dsg_golden() -> TestResult {
    let path =
        reference().join("neatwork5/examples/dsg_examples/Toy/Toy_bbd_Secu1.00_fusion0_split0.dsg");
    let original = String::from_utf8_lossy(&std::fs::read(&path)?).to_string();
    let net = read(&path)?;
    let written = write_dsg(&net);
    let mut expected = String::new();
    for line in original.lines() {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() == 8 && f[5] == "0" && f[6] == "0" {
            expected.push_str(&format!("{},fictif,{}\n", f[..6].join(","), f[7]));
        } else {
            expected.push_str(line);
            expected.push('\n');
        }
    }
    assert_eq!(written, expected);
    Ok(())
}

const SMALL_DSG: &str = "\
!Nodes
S,0,0,0,1,1,0
B,-10.5,1e-06,2,0.0044,0.0044,2
!Pipes
S,B,100,60,D002,40,D001,N
!Default properties
targetflow,0.2
bounded-flow,1
serviceequal,0.7
lambda,1.781
limitbudget,1.00E+09
!Diameters
D001,a,13.5,0.0182,1.5,200,1,0.0015,N
D002,b,PEHD,0.0235,2,170,EPANET,0,N
!Orifices
0.0044,N,N,N,N,N
!Clients
c2,B,0.5,0.2
c10,B,0.5,0.2
!Borne
B,3,N
!loadflow
B,0.4,1.25
!Hydraulics,
S,B,1.0E-3,0.4,  2.5,1.25
!Weird section
free text, with commas ,and spaces  
";

const SMALL_DSG_WRITTEN: &str = "\
!Nodes
S,0,0,0,1,1,0
B,-10.5,1e-06,2,0.0044,0.0044,2
!Pipes
S,B,100,60,D002,40,D001,N
!Default properties
bounded_flow,1
limitbudget,1000000000
opentaps,0.5
servicequal,0.7
targetflow,0.2
uniform_dem,1
lambda,1.781
!Diameters
D001,a,13.5,0.0182,1.5,200,1,0.0015,N
D002,b,PEHD,0.0235,2,170,EPANET,0,N
!Orifices
0.0044,N,N,N,N,N
!Clients
c10,B,0.5,0.2
c2,B,0.5,0.2
!Borne
B,3,N
!loadflow
B,0.4,1.25
!Hydraulics,
S,B,1.0E-3,0.4,  2.5,1.25
!Weird section
free text, with commas ,and spaces  
";

/// Canonical parameter names, sorted design then technical parameters,
/// `%.15g` numbers, sorted clients, `!loadflow` (ADR-022) and unknown
/// sections byte-identical.
#[test]
fn small_dsg_golden() -> TestResult {
    let net = read_dsg("small", SMALL_DSG)?.network;
    assert_eq!(write_dsg(&net), SMALL_DSG_WRITTEN);
    Ok(())
}

#[test]
fn small_tpo_golden() -> TestResult {
    let net = read_dsg("small", SMALL_DSG)?.network;
    let tpo = write_tpo(&net);
    let expected = "\
!Nodes
S,0,0,0,0,0
B,-10.5,1e-06,2,2,2
!Pipes
S,B,100
!Default properties
bounded_flow,1
limitbudget,1000000000
opentaps,0.5
servicequal,0.7
targetflow,0.2
uniform_dem,1
lambda,1.781
!Diameters
a,13.5,0.0182,1.5,200,1,0.0015
b,PEHD,0.0235,2,170,EPANET,0
!Orifices
0.0044,N
!Clients
c10,B,0.5,0.2
c2,B,0.5,0.2
!Borne
B,3,N
!Hydraulics,
S,B,1.0E-3,0.4,  2.5,1.25
!Weird section
free text, with commas ,and spaces  
";
    assert_eq!(tpo, expected);
    Ok(())
}

/// Generated catalogue IDs give a 7-field `!Diameters` in a `.tpo`.
#[test]
fn tpo_with_generated_ids_writes_seven_fields() -> TestResult {
    let path = reference().join("neatwork5/examples/tpo_examples/Toy/Toy_bbd.tpo");
    let net = read(&path)?;
    let tpo = write_tpo(&net);
    assert!(
        tpo.contains("\n!Diameters\n\"1/2\"\"\",13.5,0.0182,0.2799,200,1,0.0015\n"),
        "{tpo}"
    );
    assert!(tpo.contains("\n!Orifices\n0.0017,N\n"));
    assert!(tpo.starts_with("!Commentaires\nbbd = modele de base"));
    assert!(tpo.contains("\nfaucetcoef,1.87e-08\n"));
    Ok(())
}

#[test]
fn db_catalogues_round_trip() -> TestResult {
    let dir = reference().join("neatwork5/examples/tpo_examples");
    for f in [
        "Ceiba/DiamBaseCeiba.db",
        "Cruscade/DiamBaseCruscade.db",
        "Red 1/DiamBaseRed 1.db",
        "Toy/DiamBaseToy.db",
        "ToyXlx/DiamBaseToyXlx.db",
    ] {
        let a = read_diameters_db(&std::fs::read_to_string(dir.join(f))?)?;
        let text = write_diameters_db(&a);
        assert!(text.ends_with('\n'));
        let b = read_diameters_db(&text)?;
        assert_eq!(a, b, "{f}");
    }
    for f in ["Ceiba/orifices.db", "Toy/orifices.db"] {
        let a = read_orifices_db(&std::fs::read_to_string(dir.join(f))?);
        let text = write_orifices_db(&a);
        assert_eq!(read_orifices_db(&text), a, "{f}");
    }
    assert_eq!(
        write_orifices_db(&[0.0017, 1e-6, 1.0]),
        "0.0017\n1e-06\n1\n"
    );
    let toy = read_diameters_db(&std::fs::read_to_string(dir.join("Toy/DiamBaseToy.db"))?)?;
    assert!(
        write_diameters_db(&toy).starts_with("1/2\",13.5,0.0182,0.2799,200,1,0.0015\n3/4\",17,")
    );
    Ok(())
}
