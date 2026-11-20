//! `neatwork.project.Simulation`: read a `.sim` file back for display.
//!
//! The Java program finds each table by the number of fields of a line and
//! converts the cells to `Integer` or `Double`. A cell that does not convert
//! becomes an empty string.

use crate::java::{double_to_string, parse_double, parse_int, read_csv};

/// One cell of a simulation table.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Text(String),
    Int(i32),
    Double(f64),
    /// The conversion failed (Java stores "").
    Empty,
}

impl Cell {
    /// Text as the Java table shows it (`toString` of the value).
    pub fn display(&self) -> String {
        match self {
            Cell::Text(s) => s.clone(),
            Cell::Int(i) => i.to_string(),
            Cell::Double(d) => double_to_string(*d),
            Cell::Empty => String::new(),
        }
    }

    /// Numeric value, if any.
    pub fn number(&self) -> Option<f64> {
        match self {
            Cell::Int(i) => Some(f64::from(*i)),
            Cell::Double(d) => Some(*d),
            _ => None,
        }
    }
}

/// A simulation file and its tables.
#[derive(Clone, Debug, Default)]
pub struct SimulationFile {
    pub name: String,
    pub content: String,
    data: Vec<Vec<String>>,
}

impl SimulationFile {
    pub fn new(name: &str, content: &str) -> Self {
        SimulationFile {
            name: name.to_string(),
            content: content.to_string(),
            data: read_csv(content),
        }
    }

    /// `getProperties`: every 3-field line as `simu.<name>.value`.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.data
            .iter()
            .filter(|t| t.len() == 3 && t[0] == name)
            .map(|t| t[1].as_str())
            .next_back()
    }

    fn lines(&self, size: usize, types: &str) -> Vec<Vec<Cell>> {
        self.data
            .iter()
            .filter(|t| t.len() == size)
            .map(|t| {
                types
                    .chars()
                    .enumerate()
                    .map(|(i, ty)| {
                        let s = t.get(i).map(String::as_str).unwrap_or("");
                        match ty {
                            'I' => parse_int(s).map_or(Cell::Empty, Cell::Int),
                            'D' => parse_double(s).map_or(Cell::Empty, Cell::Double),
                            _ => Cell::Text(s.to_string()),
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// `getFlowTaps`: tap, runs, min, mean, max, variability, % below,
    /// % above, failures.
    pub fn flow_taps(&self) -> Vec<Vec<Cell>> {
        self.lines(10, "SIDDDDDDI")
    }

    /// `getQuartileTaps`: tap, runs, min, 10%, 25%, 50%, 75%, 90%, max.
    pub fn quartile_taps(&self) -> Vec<Vec<Cell>> {
        self.lines(11, "SIDDDDDDD")
    }

    /// `getSpeedPipe`: pipe, runs, mean speed, max speed.
    pub fn speed_pipes(&self) -> Vec<Vec<Cell>> {
        self.lines(5, "SIDD")
    }

    /// `getNodesPressure`: node, min, mean, max.
    pub fn node_pressures(&self) -> Vec<Vec<Cell>> {
        self.lines(4, "SDDD")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tables_by_field_count() {
        let s = SimulationFile::new(
            "toy.0",
            "Global average,-,-,0.117,-,?,25,0,0,-1\nP1,1,0.12,0.12,0.12,0,0,0,0,-1\nA,10,10,10\nnbsim,4,N\n",
        );
        let f = s.flow_taps();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0][1], Cell::Empty);
        assert_eq!(f[1][2].display(), "0.12");
        assert_eq!(f[1][5].display(), "0.0");
        assert_eq!(s.node_pressures()[0][1].display(), "10.0");
        assert_eq!(s.property("nbsim"), Some("4"));
    }
}
