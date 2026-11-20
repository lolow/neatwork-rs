//! Design helpers of the windows: cost (`Design.getCost`,
//! `DesignTablePane.updateCostLabel`), used lengths (`Design.getSummary`)
//! and topography extraction (`Design.extractTopoContent`).

use std::collections::HashMap;

use crate::java::decimal_format;
use crate::project::network::{TYPE_FAUCET, topo_node_csv, topo_pipe_csv};
use crate::project::{Design, Diameter};
use crate::{Error, Result};

fn diameter<'a>(design: &'a Design, reference: &str) -> Result<&'a Diameter> {
    design
        .diam_table
        .get(&reference.to_string())
        .ok_or_else(|| Error::Network(format!("unknown diameter reference {reference}")))
}

/// `Design.getCost`: sum of unit cost times length over both parts of
/// every pipe. A part with a length above 0 and an unknown reference is an
/// error (Java throws).
pub fn design_cost(design: &Design) -> Result<f64> {
    let mut cost = 0.0;
    for p in &design.network.pipes {
        if p.length1 > 0.0 {
            cost += diameter(design, &p.ref_diam1)?.cost * p.length1;
        }
        if p.length2 > 0.0 {
            cost += diameter(design, &p.ref_diam2)?.cost * p.length2;
        }
    }
    Ok(cost)
}

/// `Tools.doubleFormat("#,##0", d)`: rounded half to even, digits grouped
/// by three with `,` (`Locale.US`).
pub fn format_grouped(d: f64) -> String {
    let s = decimal_format("0", d);
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", s.as_str()),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return s;
    }
    let mut out = String::from(sign);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Text of the cost label under the design tables
/// (`DesignTablePane.updateCostLabel`).
pub fn design_cost_label(cost: f64) -> String {
    format!("Design total cost: {}", format_grouped(cost))
}

/// `Design.getSummary`: used length of each diameter reference. Both
/// references of a pipe count, whatever the part length; reference `0`
/// is left out.
pub fn used_lengths(design: &Design) -> HashMap<String, f64> {
    let mut h: HashMap<String, f64> = HashMap::new();
    for p in &design.network.pipes {
        for (reference, length) in [(&p.ref_diam1, p.length1), (&p.ref_diam2, p.length2)] {
            let used = h.entry(reference.clone()).or_insert(0.0);
            if reference != "0" {
                *used += length;
            }
        }
    }
    h.remove("0");
    h
}

/// `Design.extractTopoContent`: the `.tpo` text of a design, without the
/// `!Nodes` and `!Pipes` headers (as in Java). A node `X_a` is a tap of
/// `X`: it is left out and counts as one faucet of `X`; its pipes are
/// left out too.
///
/// The Java method also writes these tap counts into the nodes of the
/// design, so a later design report counts them as individual faucets.
/// This function does not change the design.
pub fn extract_topo_content(design: &Design) -> String {
    let mut nodes = design.network.nodes.clone();
    let mut nbfaucet: HashMap<String, i32> = HashMap::new();
    for n in &mut nodes {
        n.nb_taps = i32::from(n.node_type == TYPE_FAUCET);
        if let Some(k) = n.name.find('_').filter(|k| *k > 0) {
            *nbfaucet.entry(n.name[..k].to_string()).or_insert(0) += 1;
        }
    }
    let mut content = String::new();
    for n in &mut nodes {
        if !n.name.contains('_') {
            if let Some(nb) = nbfaucet.get(&n.name) {
                n.nb_taps = *nb;
            }
            content.push_str(&topo_node_csv(n));
            content.push('\n');
        }
    }
    for p in &design.network.pipes {
        if !p.begin.contains('_') && !p.end.contains('_') {
            content.push_str(&topo_pipe_csv(p));
            content.push('\n');
        }
    }
    content.push_str("!Default properties\n");
    for name in design.properties.property_names() {
        if name.starts_with("topo.")
            && name.ends_with(".value")
            && let Some(short) = name.get(5..name.len() - 6)
        {
            let value = design.properties.get_property(&name).unwrap_or("");
            content.push_str(&format!("{short},{value}\n"));
        }
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_support::fixtures::*;
    use crate::java::double_to_string;

    #[test]
    fn cost_matches_java() {
        let expected: Vec<&str> = include_str!("testdata/cost.txt").lines().collect();
        let toy = design_cost(&design("toy", TOY_DSG)).ok();
        let el = design_cost(&design("El_Bihajua", EL_BIHAJUA_DSG)).ok();
        assert_eq!(toy.map(design_cost_label).as_deref(), Some(expected[0]));
        assert_eq!(toy.map(double_to_string).as_deref(), Some(expected[1]));
        assert_eq!(el.map(design_cost_label).as_deref(), Some(expected[2]));
        assert_eq!(el.map(double_to_string).as_deref(), Some(expected[3]));
    }

    #[test]
    fn unknown_reference_is_an_error() {
        let d = design("x", "S,0,0,0,0,0,0\nA,-1,0,0,0,0,2\nS,A,10,10,D9,0,0,N\n");
        assert!(design_cost(&d).is_err());
    }

    #[test]
    fn grouping() {
        assert_eq!(format_grouped(0.0), "0");
        assert_eq!(format_grouped(-0.2), "-0");
        assert_eq!(format_grouped(999.5), "1,000");
        assert_eq!(format_grouped(1234.5), "1,234");
        assert_eq!(format_grouped(1235.5), "1,236");
        assert_eq!(format_grouped(-1234567.0), "-1,234,567");
        assert_eq!(format_grouped(123456.0), "123,456");
        assert_eq!(format_grouped(f64::NAN), "NaN");
        assert_eq!(format_grouped(f64::INFINITY), "\u{221e}");
    }

    #[test]
    fn used_lengths_skip_reference_zero() {
        let u = used_lengths(&design("toy", TOY_DSG));
        assert_eq!(u.get("D001"), Some(&374.0));
        assert_eq!(u.get("D002"), Some(&896.0));
        assert_eq!(u.get("0"), None);
    }

    #[test]
    fn extract_matches_java() {
        assert_eq!(
            extract_topo_content(&design("toy", TOY_DSG)),
            include_str!("testdata/extract_toy_dsg.tpo")
        );
        assert_eq!(
            extract_topo_content(&design("El_Bihajua", EL_BIHAJUA_DSG)),
            include_str!("testdata/extract_El_Bihajua_dsg.tpo")
        );
    }

    #[test]
    fn extract_merges_split_faucets() {
        let dsg = "S,0,0,0,0,0,0\nA,-1,0,0,0,0,1\nA_a,-1,0,0,0,0,2\nA_b,-1,0,0,0,0,2\n\
                   S,A,10,10,D1,0,0,N\nA,A_a,1,1,D1,0,0,N\nA,A_b,1,1,D1,0,0,N\n";
        let text = extract_topo_content(&design("x", dsg));
        assert!(
            text.starts_with(
                "S,0.0,0.0,0.0,0,0\nA,-1.0,0.0,0.0,2,1\nS,A,10.0\n!Default properties\n"
            )
        );
    }
}
