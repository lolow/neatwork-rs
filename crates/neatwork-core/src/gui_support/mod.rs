//! Logic of the NeatWork 2024 windows (`neatwork.gui`) that does not draw:
//! the tree layout, the summaries, the HTML reports, the project names and
//! the table helpers.
//!
//! Each function gives the same text as the Java window with the English
//! bundle (`neatwork_en.properties`). Where the Java code would throw, the
//! function returns an error instead. Java behavior that looks wrong is kept
//! and noted at the place where it happens.

pub mod design;
pub mod naming;
pub mod reports;
pub mod stats;
pub mod table;
pub mod tree;

pub use design::{design_cost, design_cost_label, extract_topo_content, format_grouped};
pub use naming::{already_loaded_name, first_compatible_name};
pub use reports::{
    database_diameters_report, database_orifices_report, design_report, simulation_report,
    topography_report,
};
pub use stats::{NetworkStats, QuickCheck, network_stats, quick_check, topo_stat_html};
pub use table::{ColumnKind, copy_text, new_node_name, parse_paste, topo_node_name};
pub use tree::{
    TreeCell, TreeCellKind, TreeLayout, TreeView, build_tree, is_a_tree, path_to_source, search,
};

use crate::project::network::{TYPE_DISPATCH, TYPE_FAUCET, TYPE_RESERVOIR};

/// `Node.getNameType`: name of a node type in the tables and reports.
pub fn node_type_name(node_type: i32) -> &'static str {
    match node_type {
        TYPE_DISPATCH => "BRANCHING NODE",
        TYPE_FAUCET => "FAUCET NODE",
        TYPE_RESERVOIR => "TANK",
        _ => "",
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Sample projects of NeatWork 2024 and the outputs that the Java code
    //! gives on them (recorded with the Java classes, date replaced by
    //! `DATE`).

    use crate::{Design, Locale, Topographie, application_properties};

    macro_rules! sample {
        ($f:literal) => {
            include_str!(concat!(
                "../../../../reference/neatwork2024/sample_projects/",
                $f
            ))
        };
    }

    pub const EL_BIHAJUA_TPO: &str = sample!("El_Bihajua.tpo");
    pub const EL_BIHAJUA_DSG: &str = sample!("El_Bihajua.dsg");
    pub const EL_BIHAJUA_0_SIM: &str = sample!("El_Bihajua.0.sim");
    pub const SAMPLE_NETWORK_TPO: &str = sample!("SampleNetwork.tpo");
    pub const BASE_EXAMPLE_TPO: &str = sample!("baseexample.tpo");
    pub const TOY_DSG: &str = sample!("toy.dsg");
    pub const TOY_0_SIM: &str = sample!("toy.0.sim");
    pub const TOY_1_SIM: &str = sample!("toy.1.sim");
    pub const TOY_3_SIM: &str = sample!("toy.3.sim");
    pub const TOY_6_SIM: &str = sample!("toy.6.sim");

    pub fn topo(name: &str, text: &str) -> Topographie {
        Topographie::new(name, text, &application_properties(Locale::En), Locale::En)
    }

    pub fn design(name: &str, text: &str) -> Design {
        Design::new(name, text, &application_properties(Locale::En), Locale::En)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_names() {
        assert_eq!(node_type_name(TYPE_RESERVOIR), "TANK");
        assert_eq!(node_type_name(TYPE_DISPATCH), "BRANCHING NODE");
        assert_eq!(node_type_name(TYPE_FAUCET), "FAUCET NODE");
        assert_eq!(node_type_name(7), "");
    }
}
