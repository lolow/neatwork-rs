//! Table helpers of `gui.fancytable.FancyTableModel` and the node table
//! models (`TopoNodeTableModel`, `DesignNodeModel`).

use crate::java::{parse_double, parse_int};
use crate::project::Cell;

/// Text of the error dialog when a paste does not fit the table.
pub const CLIPBOARD_NOT_WELL_FORMATED: &str = "Clipboard is not well formated";

/// Title of that dialog.
pub const FORMAT_ERROR: &str = "Error in format";

/// Class of a table column, from the row that `getNewVector` builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnKind {
    Text,
    Double,
    Int,
}

/// `getFirstPossibleName("N")` of the node tables: `N`, else `N1`, `N2`...
pub fn new_node_name(existing_names: &[String]) -> String {
    let mut name = "N".to_string();
    let mut cpt = 0;
    while existing_names.contains(&name) {
        cpt += 1;
        name = format!("N{cpt}");
    }
    name
}

/// `TopoNodeTableModel.setValueAt` on the name column: an edited name is
/// cut at its first `_`, unless the `_` is the first character.
pub fn topo_node_name(value: &str) -> String {
    match value.find('_') {
        Some(k) if k > 0 => value[..k].to_string(),
        _ => value.to_string(),
    }
}

/// `FancyTableModel.getCopy`: the first `column_count` cells of each row,
/// separated by TAB, each row ended by `\n`.
/// A missing cell is copied as empty text (Java throws).
pub fn copy_text(rows: &[Vec<Cell>], column_count: usize) -> String {
    let mut copy = String::new();
    for r in rows {
        let cells: Vec<String> = (0..column_count)
            .map(|j| r.get(j).map(Cell::display).unwrap_or_default())
            .collect();
        copy.push_str(&cells.join("\t"));
        copy.push('\n');
    }
    copy
}

/// `FancyTableModel.setPaste`: split on `\n` then TAB, empty pieces
/// dropped. Each line must have one cell per column of `kinds`; cells are
/// converted to the column class.
///
/// Errors: [`CLIPBOARD_NOT_WELL_FORMATED`] when a line has another number of
/// cells; [`FORMAT_ERROR`] when a number does not parse (Java throws
/// `NumberFormatException` there, after it has emptied the table).
pub fn parse_paste(clip: &str, kinds: &[ColumnKind]) -> Result<Vec<Vec<Cell>>, String> {
    let lines: Vec<Vec<&str>> = clip
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').filter(|c| !c.is_empty()).collect())
        .collect();
    if lines.iter().any(|l| l.len() != kinds.len()) {
        return Err(CLIPBOARD_NOT_WELL_FORMATED.to_string());
    }
    lines
        .iter()
        .map(|l| {
            l.iter()
                .zip(kinds)
                .map(|(cell, kind)| match kind {
                    ColumnKind::Text => Some(Cell::Text(cell.to_string())),
                    ColumnKind::Double => parse_double(cell).map(Cell::Double),
                    ColumnKind::Int => parse_int(cell).map(Cell::Int),
                })
                .collect::<Option<Vec<Cell>>>()
                .ok_or_else(|| FORMAT_ERROR.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_names() {
        assert_eq!(new_node_name(&[]), "N");
        let used: Vec<String> = ["A", "N", "N1", "N3"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(new_node_name(&used), "N2");
        assert_eq!(topo_node_name("P1_a"), "P1");
        assert_eq!(topo_node_name("P1_a_b"), "P1");
        assert_eq!(topo_node_name("_P1"), "_P1");
        assert_eq!(topo_node_name("P1"), "P1");
    }

    #[test]
    fn copy() {
        let rows = vec![
            vec![
                Cell::Text("A".into()),
                Cell::Double(-10.0),
                Cell::Int(1),
                Cell::Text("x".into()),
            ],
            vec![
                Cell::Text("B".into()),
                Cell::Double(1e-6),
                Cell::Int(2),
                Cell::Text("y".into()),
            ],
        ];
        assert_eq!(copy_text(&rows, 3), "A\t-10.0\t1\nB\t1.0E-6\t2\n");
        assert_eq!(copy_text(&[], 3), "");
    }

    #[test]
    fn paste() {
        use ColumnKind::*;
        let kinds = [Text, Double, Int];
        assert_eq!(
            parse_paste("A\t-1.5\t2\nB\t3d\n", &kinds),
            Err(CLIPBOARD_NOT_WELL_FORMATED.to_string())
        );
        assert_eq!(
            parse_paste("A\t-1.5\t2\n\n\tB\t3d\t\t4\r", &kinds),
            Err(FORMAT_ERROR.to_string())
        );
        assert_eq!(
            parse_paste("A\t-1.5\t2\n\n\tB\t3d\t\t4\n", &kinds),
            Ok(vec![
                vec![Cell::Text("A".into()), Cell::Double(-1.5), Cell::Int(2)],
                vec![Cell::Text("B".into()), Cell::Double(3.0), Cell::Int(4)],
            ])
        );
        // The last line keeps its `\r` in a text cell.
        assert_eq!(
            parse_paste("x\r\n", &[Text]),
            Ok(vec![vec![Cell::Text("x\r".into())]])
        );
        assert_eq!(parse_paste("", &kinds), Ok(vec![]));
        assert_eq!(
            parse_paste("A\t1\n", &kinds),
            Err(CLIPBOARD_NOT_WELL_FORMATED.to_string())
        );
    }
}
