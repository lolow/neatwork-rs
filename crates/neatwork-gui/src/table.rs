//! Editable table of the Java `FancyTable`: rows of raw values, typed
//! columns, selection, insert, delete, copy and paste. The project changes
//! only when the pane applies the table.

use std::collections::BTreeSet;

use neatwork_core::java::{double_to_string, parse_double, parse_int};
use slint::{ModelRc, SharedString, VecModel};

use crate::{Column, Row, TableData};

/// Type of the values of a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Double,
    Int,
    /// Node type: stored as 0/1/2, shown as its name.
    NodeType,
    /// Orifice diameter: 10000 is "none" and shows `-`.
    Orifice,
    /// Diameter reference of a design pipe, chosen in a list.
    Diameter,
    /// Check box: "true" or "false".
    Check,
    /// Percentage with a colored bar.
    Bar,
}

#[derive(Clone, Debug)]
pub struct Col {
    pub title: String,
    pub width: f32,
    pub editable: bool,
    pub kind: Kind,
}

impl Col {
    pub fn new(title: String, width: f32, editable: bool, kind: Kind) -> Self {
        Col {
            title,
            width,
            editable,
            kind,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub cols: Vec<Col>,
    pub rows: Vec<Vec<String>>,
    pub selected: BTreeSet<usize>,
    anchor: Option<usize>,
    pub changed: bool,
    pub can_copy: bool,
    /// Edits turn the table gray (topography, design and database tables).
    pub track_changes: bool,
}

/// Result of an edit.
pub enum Edit {
    Done,
    /// The text is not a valid value for the column.
    Invalid,
}

impl Table {
    pub fn new(cols: Vec<Col>, track_changes: bool) -> Self {
        Table {
            cols,
            can_copy: true,
            track_changes,
            ..Table::default()
        }
    }

    /// `updateData`: new rows from the project, no pending edit.
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        self.selected.clear();
        self.anchor = None;
        self.changed = false;
        self.can_copy = true;
    }

    fn mark_changed(&mut self) {
        if self.track_changes {
            self.changed = true;
        }
        self.can_copy = false;
    }

    /// Click on a row header: select it; Ctrl toggles, Shift extends.
    pub fn click(&mut self, row: usize, ctrl: bool, shift: bool) {
        if row >= self.rows.len() {
            return;
        }
        if shift && let Some(a) = self.anchor {
            let (lo, hi) = if a <= row { (a, row) } else { (row, a) };
            if !ctrl {
                self.selected.clear();
            }
            self.selected.extend(lo..=hi);
            return;
        }
        if ctrl {
            if !self.selected.remove(&row) {
                self.selected.insert(row);
            }
        } else {
            self.selected.clear();
            self.selected.insert(row);
        }
        self.anchor = Some(row);
    }

    /// Set a cell from the text the user typed. Numbers are stored as Java
    /// writes them (`Double.toString`), so the applied text is the same.
    pub fn edit(&mut self, row: usize, col: usize, text: &str) -> Edit {
        let Some(c) = self.cols.get(col) else {
            return Edit::Invalid;
        };
        let value = match c.kind {
            Kind::Double | Kind::Bar => match parse_double(text) {
                Some(d) => double_to_string(d),
                None => return Edit::Invalid,
            },
            Kind::Int | Kind::NodeType => match parse_int(text.trim()) {
                Some(i) => i.to_string(),
                None => return Edit::Invalid,
            },
            // DesignNodeModel: a text with '-' means "no orifice".
            Kind::Orifice => {
                if text.contains('-') {
                    double_to_string(10000.0)
                } else {
                    match parse_double(text) {
                        Some(d) => double_to_string(d),
                        None => return Edit::Invalid,
                    }
                }
            }
            Kind::Text | Kind::Diameter | Kind::Check => text.to_string(),
        };
        let Some(r) = self.rows.get_mut(row) else {
            return Edit::Invalid;
        };
        if col < r.len() {
            r[col] = value;
        }
        if c.kind != Kind::Check {
            self.mark_changed();
        }
        Edit::Done
    }

    /// `FancyTableModel.insertRow`: after the first selected row, or at the
    /// end. A row never goes to position 0.
    pub fn insert(&mut self, row: Vec<String>) -> usize {
        let at = self.selected.iter().next().map_or(0, |s| s + 1);
        let at = if at < 1 || at > self.rows.len() {
            self.rows.len()
        } else {
            at
        };
        self.rows.insert(at, row);
        self.selected.clear();
        self.selected.insert(at);
        self.anchor = Some(at);
        self.mark_changed();
        at
    }

    /// Delete the selected rows.
    pub fn delete(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        for r in self.selected.iter().rev() {
            if *r < self.rows.len() {
                self.rows.remove(*r);
            }
        }
        self.selected.clear();
        self.anchor = None;
        self.mark_changed();
    }

    /// Number of visible columns (a design pipe row has a hidden "N").
    fn visible(&self) -> usize {
        self.cols.len()
    }

    /// Copy: the whole table, tab between cells, a new line after each row.
    pub fn copy_text(&self) -> String {
        let n = self.visible();
        self.rows
            .iter()
            .map(|r| r.iter().take(n).cloned().collect::<Vec<_>>().join("\t") + "\n")
            .collect()
    }

    /// Paste: replace the whole table. Each line needs one value per
    /// column; numbers must parse. `extra` values are appended to each row
    /// (the hidden fields).
    pub fn paste(&mut self, text: &str, extra: &[&str]) -> Result<(), ()> {
        let n = self.visible();
        let mut rows = Vec::new();
        for line in text.split('\n').filter(|l| !l.is_empty()) {
            let line = line.trim_end_matches('\r');
            let cells: Vec<&str> = line.split('\t').filter(|c| !c.is_empty()).collect();
            if cells.len() != n {
                return Err(());
            }
            let mut row = Vec::with_capacity(n + extra.len());
            for (c, cell) in self.cols.iter().zip(&cells) {
                let v = match c.kind {
                    Kind::Double | Kind::Orifice | Kind::Bar => {
                        double_to_string(parse_double(cell).ok_or(())?)
                    }
                    Kind::Int | Kind::NodeType => parse_int(cell.trim()).ok_or(())?.to_string(),
                    _ => (*cell).to_string(),
                };
                row.push(v);
            }
            row.extend(extra.iter().map(|s| (*s).to_string()));
            rows.push(row);
        }
        self.rows = rows;
        self.selected.clear();
        self.anchor = None;
        if self.track_changes {
            self.changed = true;
        }
        Ok(())
    }

    /// Set every check box of the selected rows (all rows when none is
    /// selected), as the "All" and "None" buttons do.
    pub fn set_checks(&mut self, col: usize, value: bool) {
        let all = self.selected.is_empty();
        for (i, r) in self.rows.iter_mut().enumerate() {
            if (all || self.selected.contains(&i)) && col < r.len() {
                r[col] = value.to_string();
            }
        }
    }

    /// Model for the `DataTable` component. `show` gives the text of a cell
    /// and `locked` tells if a cell is read-only.
    pub fn to_slint(
        &self,
        show: &dyn Fn(usize, usize, &str) -> String,
        locked: &dyn Fn(usize, usize) -> bool,
    ) -> TableData {
        let mut x = 0.0;
        let columns: Vec<Column> = self
            .cols
            .iter()
            .map(|c| {
                let col = Column {
                    title: SharedString::from(c.title.as_str()),
                    x,
                    width: c.width,
                    editable: c.editable,
                    kind: match c.kind {
                        Kind::Diameter => 1,
                        Kind::Check => 2,
                        Kind::Bar => 3,
                        _ => 0,
                    },
                };
                x += c.width;
                col
            })
            .collect();
        let n = self.visible();
        let rows: Vec<Row> = self
            .rows
            .iter()
            .enumerate()
            .map(|(r, raw)| {
                let cells: Vec<SharedString> = (0..n)
                    .map(|c| SharedString::from(show(r, c, raw.get(c).map_or("", String::as_str))))
                    .collect();
                let lock: Vec<bool> = (0..n).map(|c| locked(r, c)).collect();
                Row {
                    cells: ModelRc::new(VecModel::from(cells)),
                    selected: self.selected.contains(&r),
                    locked: ModelRc::new(VecModel::from(lock)),
                }
            })
            .collect();
        TableData {
            columns: ModelRc::new(VecModel::from(columns)),
            rows: ModelRc::new(VecModel::from(rows)),
            changed: self.changed,
            can_copy: self.can_copy,
        }
    }

    /// Model with the raw values shown as they are.
    pub fn plain(&self) -> TableData {
        self.to_slint(&|_, _, v| v.to_string(), &|_, _| false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Table {
        let mut t = Table::new(
            vec![
                Col::new("ID".into(), 100.0, true, Kind::Text),
                Col::new("Height".into(), 100.0, true, Kind::Double),
            ],
            true,
        );
        t.set_rows(vec![
            vec!["A".into(), "1.0".into()],
            vec!["B".into(), "2.0".into()],
        ]);
        t
    }

    #[test]
    fn insert_after_selection_or_at_end() {
        let mut t = t();
        assert_eq!(t.insert(vec!["C".into(), "0.0".into()]), 2);
        t.click(0, false, false);
        assert_eq!(t.insert(vec!["D".into(), "0.0".into()]), 1);
        assert!(t.changed && !t.can_copy);
    }

    #[test]
    fn edit_normalizes_numbers() {
        let mut t = t();
        assert!(matches!(t.edit(0, 1, "-12"), Edit::Done));
        assert_eq!(t.rows[0][1], "-12.0");
        assert!(matches!(t.edit(0, 1, "abc"), Edit::Invalid));
    }

    #[test]
    fn copy_and_paste() {
        let mut t = t();
        assert_eq!(t.copy_text(), "A\t1.0\nB\t2.0\n");
        assert!(t.paste("X\t3\r\nY\t4\n", &[]).is_ok());
        assert_eq!(t.rows, vec![vec!["X", "3.0"], vec!["Y", "4.0"]]);
        assert!(t.paste("X\n", &[]).is_err());
    }

    #[test]
    fn selection_with_modifiers() {
        let mut t = t();
        t.insert(vec!["C".into(), "0.0".into()]);
        t.click(0, false, false);
        t.click(2, false, true);
        assert_eq!(t.selected.len(), 3);
        t.click(1, true, false);
        assert_eq!(t.selected.len(), 2);
    }
}
