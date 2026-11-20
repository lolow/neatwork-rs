//! HTML reports of the windows: topography and design (`FrameNeatwork`),
//! simulation (`SimulationPane`) and catalogs (`DatabaseDialog`).
//!
//! The Java code writes `new Date().toString()` in some reports, for
//! example `Thu Nov 20 14:03:00 CET 2026`. Here the caller passes that text.

use super::design::{design_cost, used_lengths};
use super::node_type_name;
use super::stats::network_stats;
use crate::Result;
use crate::java::{compare_utf16, decimal_format, double_to_string};
use crate::project::simulation::Cell;
use crate::project::{Database, Design, SimulationFile, Topographie};

const TABLE: &str = "<table BORDER CELLPADDING=0 CELLSPACING=0>";

/// `<tr><td>a</td>...</tr>`.
fn row<S: AsRef<str>>(cells: impl IntoIterator<Item = S>) -> String {
    let mut s = String::from("<tr>");
    for c in cells {
        s.push_str("<td>");
        s.push_str(c.as_ref());
        s.push_str("</td>");
    }
    s.push_str("</tr>");
    s
}

/// A property as Java string concatenation shows it (`null` when absent).
fn or_null(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

/// `FrameNeatwork` topography report ("Report in HTML...").
pub fn topography_report(topo: &Topographie, date: &str) -> String {
    let mut s = String::from("<html><head><title>Topography report</title></head><body>");
    s.push_str(&format!("<h1>Topography report:{}</h1>", topo.name));
    s.push_str("<h2>Node List</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>ID</th><th>Height</th><th>X</th><th>Y</th><th>Faucets</th><th>Nature</th></tr>",
    );
    for n in &topo.network.nodes {
        s.push_str(&row([
            n.name.clone(),
            double_to_string(n.height),
            double_to_string(n.coord_x),
            double_to_string(n.coord_y),
            n.nb_taps.to_string(),
            node_type_name(n.node_type).to_string(),
        ]));
    }
    s.push_str("</table><h2>Arc List</h2>");
    s.push_str(TABLE);
    s.push_str("<tr><th>Begin</th><th>End</th><th>Length</th></tr>");
    for p in &topo.network.pipes {
        s.push_str(&row([
            p.begin.clone(),
            p.end.clone(),
            double_to_string(p.length),
        ]));
    }
    s.push_str("</table><h2>Summary</h2>");
    let st = network_stats(&topo.network);
    s.push_str(&format!(
        "<B>Number of Nodes:</B>{}<UL>\
         <LI> <i>Branching nodes:</i> {}\
         <LI> <i>Faucet nodes:</i>{} (with {} individual faucets)</UL>\
         <B>Total height change:</B> {} m<BR><BR>\
         <B>Number of pipes:</B> {} <BR>\
         <B>Total length:</B> {} m ",
        st.nb_nodes,
        st.nb_branching,
        st.nb_faucets,
        st.nb_taps,
        decimal_format("#", st.total_height_change),
        st.nb_pipes,
        decimal_format("#", st.total_length),
    ));
    s.push_str(&format!("<p>{date}</p>"));
    s
}

/// `FrameNeatwork` design report ("Report in HTML...").
///
/// The diameter table lists every reference of the design sorted by name,
/// with the length that the pipes use and its cost. The structure line
/// has two spaces before "individual faucets" (the Java code adds one).
/// It fails when a pipe part uses an unknown reference (Java throws).
pub fn design_report(design: &Design, date: &str) -> Result<String> {
    let mut s = String::from("<html><head><title>Design report</title></head><body>");
    s.push_str(&format!("<h1>Design report: {}</h1>", design.name));
    s.push_str("<h2>Node List</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>ID</th><th>Height</th><th>X</th><th>Y</th><th>Ideal Orifice</th>\
         <th>Commercial Orifice</th><th>Nature</th></tr>",
    );
    for n in &design.network.nodes {
        s.push_str(&row([
            n.name.clone(),
            double_to_string(n.height),
            double_to_string(n.coord_x),
            double_to_string(n.coord_y),
            double_to_string(n.orifice),
            double_to_string(n.orifice_commercial),
            node_type_name(n.node_type).to_string(),
        ]));
    }
    s.push_str("</table><h2>Arc List</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>Begin</th><th>End</th><th>Length</th><th>Length1</th><th>Diam1</th>\
         <th>Length2</th><th>Diam2</th></tr>",
    );
    for p in &design.network.pipes {
        s.push_str(&row([
            p.begin.clone(),
            p.end.clone(),
            double_to_string(p.length),
            double_to_string(p.length1),
            p.ref_diam1.clone(),
            double_to_string(p.length2),
            p.ref_diam2.clone(),
        ]));
    }
    s.push_str("</table>");

    let used = used_lengths(design);
    s.push_str("<h2>Diameter references</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>Ref</th><th>Nominal</th><th>SDR</th><th>Internal Diameter</th>\
         <th>Unit cost</th><th>Max Pressure</th><th>Type</th><th>Roughness</th>\
         <th>Total length</th><th>Total cost</th></tr>",
    );
    let mut refs: Vec<(&String, _)> = design.diam_table.entries().collect();
    refs.sort_by(|a, b| compare_utf16(a.0, b.0));
    for (k, d) in refs {
        let l = used.get(k).copied().unwrap_or(0.0);
        s.push_str(&row([
            k.clone(),
            d.nominal.clone(),
            double_to_string(d.sdr),
            double_to_string(d.diameter),
            decimal_format("0.##", d.cost),
            double_to_string(d.max_length),
            d.pipe_type.to_string(),
            double_to_string(d.roughness),
            double_to_string(l),
            double_to_string(l * d.cost),
        ]));
    }
    s.push_str("</table>");

    if !design.orifices.is_empty() {
        s.push_str("<h2>Available orifices</h2>");
        s.push_str(TABLE);
        s.push_str("<tr><th>Diameter</th></tr>");
        for o in &design.orifices {
            s.push_str(&row([o]));
        }
        s.push_str("</table>");
    }

    s.push_str("<h2>Summary</h2>");
    s.push_str(&format!(
        "<h3>Project cost:{}</h3>",
        decimal_format("0", design_cost(design)?)
    ));
    let p = |name: &str| {
        or_null(
            design
                .properties
                .get_property(&format!("topo.{name}.value")),
        )
    };
    s.push_str("<h3>Global parameter settings</h3>");
    s.push_str(&format!(
        "Water Temperature (\u{b0}C) : {}<br>",
        p("watertemp")
    ));
    s.push_str(&format!(
        "Pipe Commercial lengths (m) : {}<br>",
        p("pipelength")
    ));
    s.push_str("<h3>Design Parameter</h3>");
    s.push_str(&format!("Fraction of open faucets : {}<br>", p("opentaps")));
    s.push_str(&format!("Service Quality : {}<br>", p("servicequal")));
    s.push_str(&format!("Target Flow (l/s) : {}<br>", p("targetflow")));
    s.push_str(&format!("Limit on budget : {}<br>", p("limitbudget")));
    s.push_str("<h3>Advanced parameter settings</h3>");
    s.push_str(&format!("Orifice Coefficient : {}<br>", p("orifcoef")));
    s.push_str(&format!("Faucet Coefficient : {}<br>", p("faucetcoef")));
    s.push_str("<h3>Structure</h3>");
    let st = network_stats(&design.network);
    s.push_str(&format!(
        "<B>Number of Nodes:</B>{}<UL>\
         <LI> <i>Branching nodes:</i> {}\
         <LI> <i>Faucet nodes:</i>{} (with {}  individual faucets)</UL>\
         <B>Total height change:</B> {} m<BR><BR>\
         <B>Number of pipes:</B> {} <BR>\
         <B>Total length:</B> {} m ",
        st.nb_nodes,
        st.nb_branching,
        st.nb_faucets,
        st.nb_taps,
        decimal_format("#", st.total_height_change),
        st.nb_pipes,
        decimal_format("#", st.total_length),
    ));
    s.push_str(&format!("<p>{date}</p>"));
    Ok(s)
}

/// `SimulationPane` report of a simulation of the design `design_name`.
/// This report has no date.
///
/// Java bug kept: the "Speed in pipes" table shows the first four columns
/// of the percentile table (faucet, runs, min, 10%) instead of the speeds.
pub fn simulation_report(simu: &SimulationFile, design_name: &str) -> String {
    let typesimu = simu.property("typesimu");
    let mut s = String::from("<html><head><title>Simulation report</title></head><body>");
    s.push_str(&format!("<h1>Simulation report: {}</h1>", simu.name));
    s.push_str("<h2>Parameters</h2>");
    s.push_str(&format!("Simulated design: {design_name}<br>"));
    s.push_str(&format!("Type of simulation: {}", or_null(typesimu)));
    // Java calls equals on the property and throws when it is absent; here
    // an absent type shows neither line.
    if typesimu == Some("random") {
        s.push_str(&format!(
            "<br>Number of simulations: {}",
            or_null(simu.property("nbsim"))
        ));
        s.push_str(&format!(
            "<br>Fraction of open faucets: {}",
            or_null(simu.property("simopentaps"))
        ));
    }
    if typesimu == Some("tapbytap") {
        s.push_str(&format!(
            "<br>Number of simulations: {}",
            or_null(simu.property("nbsim"))
        ));
    }
    s.push_str(&format!(
        "<br>Orifices in use: {}",
        or_null(simu.property("typeorifice"))
    ));
    let table = |rows: &[Vec<Cell>], columns: Option<usize>| -> String {
        rows.iter()
            .map(|r| {
                let n = columns.map_or(r.len(), |c| c.min(r.len()));
                row(r[..n].iter().map(Cell::display))
            })
            .collect()
    };

    s.push_str("<h2>Flows at faucet</h2>");
    s.push_str(TABLE);
    s.push_str(&format!(
        "<tr><th>Faucet ID</th><th>Nb of occurences</th><th>Min</th>\
         <th>Average</th><th>Max</th><th>Variability</th><th>&lt; {}</th>\
         <th>&gt; {}</th><th>nb of failures</th></tr>",
        simu.property("mincriticalflow").unwrap_or(""),
        simu.property("maxcriticalflow").unwrap_or(""),
    ));
    s.push_str(&table(&simu.flow_taps(), None));
    s.push_str("</table>");

    s.push_str("<h2>Percentiles at faucet</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>Faucet ID</th><th>Nb of occurences</th><th>Min</th>\
         <th>&lt; 10</th><th>&lt; 25</th><th>&lt; 50</th><th>&lt; 75</th>\
         <th>&lt; 90</th><th>Max</th></tr>",
    );
    let quartiles = simu.quartile_taps();
    s.push_str(&table(&quartiles, None));
    s.push_str("</table>");

    s.push_str("<h2>Speed in pipes</h2>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>Pipe ID</th><th>Nb of simulation</th><th>Average</th>\
         <th>Maximum</th></tr>",
    );
    // Java bug kept: iterates getQuartileTaps, not getSpeedPipe.
    s.push_str(&table(&quartiles, Some(4)));
    s.push_str("</table>");

    s.push_str("<h2>Nodes Pressure</h2>");
    s.push_str(TABLE);
    s.push_str("<tr><th>Node ID</th><th>Minimum</th><th>Average</th><th>Maximum</th></tr>");
    s.push_str(&table(&simu.node_pressures(), None));
    s.push_str("</table>");
    s
}

/// `DatabaseDialog` export of the diameter catalog, sorted by diameter.
/// The internal diameter itself is not in the table (as in Java).
pub fn database_diameters_report(db: &Database, date: &str) -> String {
    let mut s = String::from("<html><head><title>Database - Diameters</title></head><body>");
    s.push_str("<h1>Database - Diameters</h1>");
    s.push_str(TABLE);
    s.push_str(
        "<tr><th>Nominal</th><th>SDR</th><th>Cost</th><th>Max Pressure</th>\
         <th>Type</th><th>Roughness</th></tr>",
    );
    for d in db.diameters() {
        s.push_str(&row([
            d.nominal.clone(),
            double_to_string(d.sdr),
            double_to_string(d.cost),
            double_to_string(d.max_length),
            d.pipe_type.to_string(),
            double_to_string(d.roughness),
        ]));
    }
    s.push_str(&format!("</table><p>{date}</p>"));
    s
}

/// `DatabaseDialog` export of the orifice catalog.
pub fn database_orifices_report(db: &Database, date: &str) -> String {
    let mut s = String::from("<html><head><title>Database - Orifices</title></head><body>");
    s.push_str("<h1>Database - Orifices</h1>");
    s.push_str(TABLE);
    s.push_str("<tr><th>Diameter</th></tr>");
    for o in db.orifices() {
        s.push_str(&row([double_to_string(*o)]));
    }
    s.push_str(&format!("</table><p>{date}</p>"));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_support::fixtures::*;

    #[test]
    fn topography_report_matches_java() {
        assert_eq!(
            topography_report(&topo("El_Bihajua", EL_BIHAJUA_TPO), "DATE"),
            include_str!("testdata/report_El_Bihajua_tpo.html")
        );
    }

    #[test]
    fn design_report_matches_java() {
        assert_eq!(
            design_report(&design("toy", TOY_DSG), "DATE")
                .ok()
                .as_deref(),
            Some(include_str!("testdata/report_toy_dsg.html"))
        );
        assert_eq!(
            design_report(&design("El_Bihajua", EL_BIHAJUA_DSG), "DATE")
                .ok()
                .as_deref(),
            Some(include_str!("testdata/report_El_Bihajua_dsg.html"))
        );
    }

    #[test]
    fn simulation_report_matches_java() {
        let cases = [
            (
                "toy.0",
                TOY_0_SIM,
                "toy",
                include_str!("testdata/report_toy_0_sim.html"),
            ),
            (
                "toy.1",
                TOY_1_SIM,
                "toy",
                include_str!("testdata/report_toy_1_sim.html"),
            ),
            (
                "toy.3",
                TOY_3_SIM,
                "toy",
                include_str!("testdata/report_toy_3_sim.html"),
            ),
            (
                "toy.6",
                TOY_6_SIM,
                "toy",
                include_str!("testdata/report_toy_6_sim.html"),
            ),
            (
                "El_Bihajua.0",
                EL_BIHAJUA_0_SIM,
                "El_Bihajua",
                include_str!("testdata/report_El_Bihajua_0_sim.html"),
            ),
        ];
        for (name, text, design, expected) in cases {
            let simu = SimulationFile::new(name, text);
            assert_eq!(simulation_report(&simu, design), expected);
        }
    }

    #[test]
    fn simulation_report_without_parameters() {
        let simu = SimulationFile::new("s", "");
        let html = simulation_report(&simu, "d");
        assert!(html.contains("Type of simulation: null<br>Orifices in use: null<h2>"));
        assert!(html.contains("<th>&lt; </th><th>&gt; </th>"));
    }

    #[test]
    fn database_reports_match_java() {
        let db = Database::default();
        assert_eq!(
            database_diameters_report(&db, "DATE"),
            include_str!("testdata/database_diameters.html")
        );
        assert_eq!(
            database_orifices_report(&db, "DATE"),
            include_str!("testdata/database_orifices.html")
        );
    }
}
