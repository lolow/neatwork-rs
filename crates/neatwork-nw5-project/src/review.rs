//! Simulation review (`specs/app.md` F41, F43–F47): the four result tables
//! of a run as plain row data (texts for display, raw values for sorting),
//! the sort rules, the run summary and the execution details.

use std::cmp::Ordering;

use crate::format::{DisplayUnit, MISSING, adaptive, fixed, integer, natural_cmp, ratio_percent};
use crate::json::{Json, compact, get_array, get_f64, get_str, n, scalar_text};
use crate::run::{RunRecord, RunStatus};

/// The four result tables (F45).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    /// Terminal Satisfaction.
    TerminalSatisfaction,
    /// Source Flow.
    SourceFlow,
    /// Pipe Velocity.
    PipeVelocity,
    /// Pressure (terminal and transit nodes).
    Pressure,
}

impl TableKind {
    /// All tables in the order of the selector.
    pub const ALL: [TableKind; 4] = [
        TableKind::TerminalSatisfaction,
        TableKind::SourceFlow,
        TableKind::PipeVelocity,
        TableKind::Pressure,
    ];

    /// Key (`terminalSatisfaction`, `sourceFlow`, `pipeVelocity`,
    /// `pressure`).
    pub fn key(self) -> &'static str {
        match self {
            TableKind::TerminalSatisfaction => "terminalSatisfaction",
            TableKind::SourceFlow => "sourceFlow",
            TableKind::PipeVelocity => "pipeVelocity",
            TableKind::Pressure => "pressure",
        }
    }

    /// Table of a key.
    pub fn parse(key: &str) -> Option<TableKind> {
        TableKind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Title.
    pub fn title(self) -> &'static str {
        match self {
            TableKind::TerminalSatisfaction => "Terminal Satisfaction",
            TableKind::SourceFlow => "Source Flow",
            TableKind::PipeVelocity => "Pipe Velocity",
            TableKind::Pressure => "Pressure",
        }
    }

    /// Description.
    pub fn description(self) -> &'static str {
        match self {
            TableKind::TerminalSatisfaction => {
                "Percentage of requested terminal demand satisfied across successful scenarios."
            }
            TableKind::SourceFlow => "Flow statistics at network sources.",
            TableKind::PipeVelocity => "Velocity statistics for network pipes.",
            TableKind::Pressure => "Pressure-head statistics at terminal and transit nodes.",
        }
    }
}

/// Semantic role of a node (F28).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    /// Status 0.
    Source,
    /// Status 1.
    Junction,
    /// Status 2.
    Terminal,
}

impl NodeRole {
    /// `SOURCE`, `JUNCTION`, `TERMINAL`.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeRole::Source => "SOURCE",
            NodeRole::Junction => "JUNCTION",
            NodeRole::Terminal => "TERMINAL",
        }
    }

    /// Label "Source", "Junction", "Terminal".
    pub fn label(self) -> &'static str {
        match self {
            NodeRole::Source => "Source",
            NodeRole::Junction => "Junction",
            NodeRole::Terminal => "Terminal",
        }
    }

    /// Role of a stored code.
    pub fn parse(text: &str) -> Option<NodeRole> {
        match text {
            "SOURCE" => Some(NodeRole::Source),
            "JUNCTION" => Some(NodeRole::Junction),
            "TERMINAL" => Some(NodeRole::Terminal),
            _ => None,
        }
    }
}

/// How a column formats its values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellFormat {
    /// Text as is.
    Text,
    /// Adaptive number (F54), converted to the display unit when the unit is
    /// m or m/s.
    Number,
    /// Ratio as a percentage.
    Percent,
    /// Integer.
    Integer,
}

/// A column of a result table.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultColumn {
    /// Key in the statistics row.
    pub key: &'static str,
    /// Label.
    pub label: &'static str,
    /// Unit shown next to the label (already in the display unit).
    pub unit: Option<String>,
    /// Format.
    pub format: CellFormat,
    /// Shown only with "Show details".
    pub details_only: bool,
    /// Header tooltip.
    pub tooltip: Option<&'static str>,
}

impl ResultColumn {
    /// Header text: "Minimum (%)", "Terminal", ...
    pub fn header(&self) -> String {
        match &self.unit {
            Some(u) => format!("{} ({u})", self.label),
            None => self.label.to_string(),
        }
    }

    /// The column compares numbers when sorting.
    pub fn numeric(&self) -> bool {
        self.format != CellFormat::Text
    }
}

/// Raw value of a cell, for sorting.
#[derive(Clone, Debug, PartialEq)]
pub enum SortValue {
    /// Missing value (sorts last in both directions).
    Empty,
    /// Number (canonical unit).
    Number(f64),
    /// Text (natural order, case-insensitive).
    Text(String),
}

/// A cell: the displayed text, the raw value and the node role used to
/// color an identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultCell {
    /// Text to show.
    pub text: String,
    /// Raw value.
    pub sort: SortValue,
    /// Role of the node in an identifier cell.
    pub role: Option<NodeRole>,
}

/// A result table.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultTable {
    /// Which table.
    pub kind: TableKind,
    /// Columns (all, including the "details" ones).
    pub columns: Vec<ResultColumn>,
    /// Rows; `cells[k]` belongs to `columns[k]`.
    pub rows: Vec<Vec<ResultCell>>,
}

/// Sort direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// ↑
    Ascending,
    /// ↓
    Descending,
}

/// Sorted column of a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortState {
    /// Column index.
    pub column: usize,
    /// Direction.
    pub direction: SortDirection,
}

/// Next sort state after a click on a header: ascending, descending, then
/// the original order (F47).
pub fn next_sort(current: Option<SortState>, column: usize) -> Option<SortState> {
    match current {
        Some(SortState {
            column: c,
            direction: SortDirection::Ascending,
        }) if c == column => Some(SortState {
            column,
            direction: SortDirection::Descending,
        }),
        Some(SortState {
            column: c,
            direction: SortDirection::Descending,
        }) if c == column => None,
        _ => Some(SortState {
            column,
            direction: SortDirection::Ascending,
        }),
    }
}

/// Header icon: ↕ (not sorted), ↑, ↓.
pub fn sort_indicator(current: Option<SortState>, column: usize) -> &'static str {
    match current {
        Some(s) if s.column == column => match s.direction {
            SortDirection::Ascending => "↑",
            SortDirection::Descending => "↓",
        },
        _ => "↕",
    }
}

/// Order of two raw values; `None` when one is empty (handled by the
/// caller: empty values sort last in both directions).
pub fn compare_values(a: &SortValue, b: &SortValue) -> Option<Ordering> {
    match (a, b) {
        (SortValue::Empty, _) | (_, SortValue::Empty) => None,
        (SortValue::Number(x), SortValue::Number(y)) => Some(x.total_cmp(y)),
        (x, y) => Some(natural_cmp(&value_text(x), &value_text(y))),
    }
}

fn value_text(v: &SortValue) -> String {
    match v {
        SortValue::Empty => String::new(),
        SortValue::Number(x) => format!("{x}"),
        SortValue::Text(t) => t.clone(),
    }
}

/// Sorted row order of a list of raw values: empty values last in both
/// directions, ties in the original order (F47).
pub fn sort_order(values: &[&SortValue], direction: SortDirection) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..values.len()).collect();
    idx.sort_by(|&i, &j| {
        let (a, b) = (values[i], values[j]);
        match (a == &SortValue::Empty, b == &SortValue::Empty) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => {
                let o = compare_values(a, b).unwrap_or(Ordering::Equal);
                match direction {
                    SortDirection::Ascending => o,
                    SortDirection::Descending => o.reverse(),
                }
            }
        }
    });
    idx
}

impl ResultTable {
    /// Visible column indices ("Show details" adds the details columns).
    pub fn visible_columns(&self, show_details: bool) -> Vec<usize> {
        (0..self.columns.len())
            .filter(|&k| show_details || !self.columns[k].details_only)
            .collect()
    }

    /// Row order for a sort state. A sort on a hidden column keeps the
    /// original order (the interface resets it, F47).
    pub fn sorted_rows(&self, sort: Option<SortState>, show_details: bool) -> Vec<usize> {
        match sort {
            Some(s) if self.visible_columns(show_details).contains(&s.column) => {
                let values: Vec<&SortValue> = self
                    .rows
                    .iter()
                    .map(|r| r.get(s.column).map_or(&SortValue::Empty, |c| &c.sort))
                    .collect();
                sort_order(&values, s.direction)
            }
            _ => (0..self.rows.len()).collect(),
        }
    }

    /// Title.
    pub fn title(&self) -> &'static str {
        self.kind.title()
    }

    /// Description.
    pub fn description(&self) -> &'static str {
        self.kind.description()
    }

    /// Row count text: "`<n>` rows".
    // PROPOSAL: the row counter text of the port.
    pub fn row_count_text(&self) -> String {
        match self.rows.len() {
            1 => "1 row".to_string(),
            k => format!("{k} rows"),
        }
    }
}

/// Display unit of a canonical result unit: m → `<u>`, m/s → `<u>/s`.
pub fn display_unit_label(canonical: &str, unit: DisplayUnit) -> String {
    match canonical {
        "m" => unit.symbol().to_string(),
        "m/s" => unit.velocity_symbol(),
        other => other.to_string(),
    }
}

/// A canonical value in the display unit (m and m/s convert; flows,
/// ratios and counts do not).
pub fn to_display(value: f64, canonical: Option<&str>, unit: DisplayUnit) -> f64 {
    match canonical {
        Some("m") | Some("m/s") => unit.from_meters(value),
        _ => value,
    }
}

struct ColumnSpec {
    key: &'static str,
    label: &'static str,
    canonical_unit: Option<String>,
    format: CellFormat,
    details_only: bool,
    tooltip: Option<&'static str>,
    role: RoleOf,
}

#[derive(Clone, Copy)]
enum RoleOf {
    None,
    Fixed(NodeRole),
    Key(&'static str),
}

fn spec(key: &'static str, label: &'static str, format: CellFormat) -> ColumnSpec {
    ColumnSpec {
        key,
        label,
        canonical_unit: None,
        format,
        details_only: false,
        tooltip: None,
        role: RoleOf::None,
    }
}

impl ColumnSpec {
    fn unit(mut self, u: &str) -> Self {
        self.canonical_unit = Some(u.to_string());
        self
    }
    fn details(mut self) -> Self {
        self.details_only = true;
        self
    }
    fn tip(mut self, t: &'static str) -> Self {
        self.tooltip = Some(t);
        self
    }
    fn role(mut self, r: RoleOf) -> Self {
        self.role = r;
        self
    }
}

fn payload_unit(stats: &Json, key: &str, fallback: &str) -> String {
    stats
        .get("units")
        .and_then(|u| get_str(u, key))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn stat_rows<'a>(stats: &'a Json, key: &str) -> &'a [Json] {
    stats.get("statistics").map_or(&[], |s| get_array(s, key))
}

/// Node role of every node in the statistics (sources, transit nodes,
/// terminals).
pub fn node_roles(stats: &Json) -> Vec<(String, NodeRole)> {
    let mut out = Vec::new();
    for (key, id, role) in [
        ("sourceFlow", "sourceId", NodeRole::Source),
        ("transitPressure", "transitNodeId", NodeRole::Junction),
        ("terminalPressure", "terminalId", NodeRole::Terminal),
    ] {
        for r in stat_rows(stats, key) {
            if let Some(i) = get_str(r, id) {
                out.retain(|(k, _)| *k != i);
                out.push((i, role));
            }
        }
    }
    out
}

/// Rows of a table as JSON objects with the keys of NW5
/// (`simulationResultTables.ts`): the Pressure rows join terminal rows then
/// transit rows; Pipe Velocity rows get `startNodeType` and `endNodeType`;
/// Terminal Satisfaction rows get the demand columns.
pub fn table_rows(record: &RunRecord, kind: TableKind) -> Vec<Json> {
    let Some(stats) = &record.statistics else {
        return Vec::new();
    };
    let roles = node_roles(stats);
    let role_of = |id: &str| {
        roles
            .iter()
            .find(|(k, _)| k == id)
            .map_or(Json::Null, |(_, r)| Json::String(r.as_str().to_string()))
    };
    let with = |row: &Json, extra: Vec<(String, Json)>| -> Json {
        let mut m = match row {
            Json::Object(m) => m.clone(),
            _ => Vec::new(),
        };
        m.extend(extra);
        Json::Object(m)
    };
    match kind {
        TableKind::TerminalSatisfaction => stat_rows(stats, "terminalSatisfaction")
            .iter()
            .map(|r| {
                let id = get_str(r, "terminalId").unwrap_or_default();
                let d = record.demand.iter().find(|t| t.terminal_id == id);
                let summary = |f: &dyn Fn(&crate::run::DemandGroup) -> f64| match d
                    .map(|t| t.groups.as_slice())
                {
                    None | Some([]) => Json::Null,
                    Some([g]) => n(f(g)),
                    Some(gs) => Json::String(format!("{} groups", gs.len())),
                };
                with(
                    r,
                    vec![
                        (
                            "clientCount".into(),
                            d.map_or(Json::Null, |t| Json::Number(t.client_count as f64)),
                        ),
                        ("targetFlowSummary".into(), summary(&|g| g.target_flow_lps)),
                        (
                            "probabilitySummary".into(),
                            summary(&|g| g.open_probability),
                        ),
                    ],
                )
            })
            .collect(),
        TableKind::SourceFlow => stat_rows(stats, "sourceFlow").to_vec(),
        TableKind::PipeVelocity => stat_rows(stats, "pipeVelocity")
            .iter()
            .map(|r| {
                let a = get_str(r, "startNodeId").unwrap_or_default();
                let b = get_str(r, "endNodeId").unwrap_or_default();
                with(
                    r,
                    vec![
                        ("startNodeType".into(), role_of(&a)),
                        ("endNodeType".into(), role_of(&b)),
                    ],
                )
            })
            .collect(),
        TableKind::Pressure => {
            let mut out = Vec::new();
            for (key, id, role) in [
                ("terminalPressure", "terminalId", "TERMINAL"),
                ("transitPressure", "transitNodeId", "JUNCTION"),
            ] {
                for r in stat_rows(stats, key) {
                    let mut m = vec![
                        (
                            "nodeId".to_string(),
                            r.get(id).cloned().unwrap_or(Json::Null),
                        ),
                        ("nodeType".to_string(), Json::String(role.into())),
                    ];
                    for k in ["minPressureM", "meanPressureM", "maxPressureM"] {
                        m.push((k.to_string(), r.get(k).cloned().unwrap_or(Json::Null)));
                    }
                    out.push(Json::Object(m));
                }
            }
            out
        }
    }
}

fn column_specs(kind: TableKind, stats: &Json) -> Vec<ColumnSpec> {
    use CellFormat::*;
    match kind {
        TableKind::TerminalSatisfaction => vec![
            spec("terminalId", "Terminal", Text).role(RoleOf::Fixed(NodeRole::Terminal)),
            spec("minSat", "Minimum", Percent).unit("%"),
            spec("moySat", "Mean", Percent).unit("%"),
            spec("maxSat", "Maximum", Percent).unit("%"),
            spec("cVarSat5", "CVaR 5%", Percent).unit("%"),
            spec("cVarSat20", "CVaR 20%", Percent).unit("%"),
            spec("clientCount", "Clients", Integer).details(),
            spec("targetFlowSummary", "Target flow / client", Text).unit("L/s").details(),
            spec("probabilitySummary", "Open probability / client", Text).details(),
            spec("activeScenarioCount", "Active scenarios", Integer)
                .details()
                .tip("Scenarios in which at least one client attached to the terminal was open."),
            spec("waterShortfallCount", "Water shortfalls", Integer).details().tip(
                "Successfully solved active scenarios in which delivered flow was below the target demand.",
            ),
            spec("stdSat", "Std deviation", Percent).details(),
            spec("cvSat", "CV", Number).details(),
        ],
        TableKind::SourceFlow => {
            let u = payload_unit(stats, "sourceFlow", "L/s");
            vec![
                spec("sourceId", "Source", Text).role(RoleOf::Fixed(NodeRole::Source)),
                spec("minSt", "Minimum", Number).unit(&u),
                spec("moySt", "Mean", Number).unit(&u),
                spec("maxSt", "Maximum", Number).unit(&u),
                spec("cVarSt5", "CVaR 5%", Number).unit(&u),
                spec("cVarSt95", "CVaR 95%", Number).unit(&u),
            ]
        }
        TableKind::PipeVelocity => {
            let u = payload_unit(stats, "pipeVelocity", "m/s");
            vec![
                spec("edgeId", "Pipe", Text),
                spec("startNodeId", "From", Text).role(RoleOf::Key("startNodeType")),
                spec("endNodeId", "To", Text).role(RoleOf::Key("endNodeType")),
                spec("minVelocityMps", "Minimum", Number).unit(&u),
                spec("meanVelocityMps", "Mean", Number).unit(&u),
                spec("maxVelocityMps", "Maximum", Number).unit(&u),
            ]
        }
        TableKind::Pressure => {
            let u = payload_unit(stats, "terminalPressure", "m");
            vec![
                spec("nodeId", "Node", Text).role(RoleOf::Key("nodeType")),
                spec("nodeType", "Type", Text).role(RoleOf::Key("nodeType")),
                spec("minPressureM", "Minimum", Number).unit(&u),
                spec("meanPressureM", "Mean", Number).unit(&u),
                spec("maxPressureM", "Maximum", Number).unit(&u),
            ]
        }
    }
}

fn cell(row: &Json, c: &ColumnSpec, unit: DisplayUnit) -> ResultCell {
    let raw = row.get(c.key).unwrap_or(&Json::Null);
    let role = match c.role {
        RoleOf::None => None,
        RoleOf::Fixed(r) => Some(r),
        RoleOf::Key(k) => get_str(row, k).and_then(|t| NodeRole::parse(&t)),
    };
    let missing = || ResultCell {
        text: MISSING.to_string(),
        sort: SortValue::Empty,
        role,
    };
    if matches!(raw, Json::Null) || raw == &Json::String(String::new()) {
        return missing();
    }
    if c.format == CellFormat::Text {
        let t = scalar_text(raw).unwrap_or_default();
        // The Type column shows the role label; it sorts by the raw code.
        let text = if c.key == "nodeType" {
            NodeRole::parse(&t).map_or(t.clone(), |r| r.label().to_string())
        } else {
            t.clone()
        };
        return ResultCell {
            text,
            sort: match raw {
                Json::Number(x) => SortValue::Number(*x),
                _ => SortValue::Text(t),
            },
            role,
        };
    }
    let Some(x) = raw.as_f64().filter(|x| x.is_finite()) else {
        return missing();
    };
    let shown = to_display(x, c.canonical_unit.as_deref(), unit);
    let text = match c.format {
        CellFormat::Integer => integer(shown),
        CellFormat::Percent => ratio_percent(shown),
        _ => adaptive(shown),
    };
    ResultCell {
        text,
        sort: SortValue::Number(x),
        role,
    }
}

/// A result table of a run, `None` when the run has no statistics (F43:
/// "This simulation does not contain result tables.").
pub fn result_table(record: &RunRecord, kind: TableKind, unit: DisplayUnit) -> Option<ResultTable> {
    let stats = record.statistics.as_ref()?;
    let specs = column_specs(kind, stats);
    let rows = table_rows(record, kind)
        .iter()
        .map(|r| specs.iter().map(|c| cell(r, c, unit)).collect())
        .collect();
    Some(ResultTable {
        kind,
        columns: specs
            .iter()
            .map(|c| ResultColumn {
                key: c.key,
                label: c.label,
                unit: c
                    .canonical_unit
                    .as_deref()
                    .map(|u| display_unit_label(u, unit)),
                format: c.format,
                details_only: c.details_only,
                tooltip: c.tooltip,
            })
            .collect(),
        rows,
    })
}

/// The four tables of a run (empty without statistics).
pub fn result_tables(record: &RunRecord, unit: DisplayUnit) -> Vec<ResultTable> {
    TableKind::ALL
        .into_iter()
        .filter_map(|k| result_table(record, k, unit))
        .collect()
}

/// Label of a run status.
// PROPOSAL: status texts of the port (NW5 shows badges with the codes).
pub fn status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Pending => "Pending",
        RunStatus::Running => "Running",
        RunStatus::Completed => "Completed",
        RunStatus::CompletedWithErrors => "Completed with errors",
        RunStatus::Failed => "Failed",
        RunStatus::CancelRequested => "Cancel requested",
        RunStatus::Cancelled => "Cancelled",
    }
}

/// "Simulation #`<n>`".
pub fn run_label(run_number: u32) -> String {
    format!("Simulation #{run_number}")
}

/// Run summary card and result header (F44).
#[derive(Clone, Debug, PartialEq)]
pub struct RunSummary {
    /// "Simulation #`<n>`".
    pub title: String,
    /// Badge "Default" / "Manual".
    pub configuration: &'static str,
    /// "Enabled" / "Disabled" / "—".
    pub bounded_flow: String,
    /// "Ideal" / "Commercial" / "—".
    pub orifice_model: String,
    /// "Per client" or "<2 decimals> L/s".
    pub target_flow: String,
    /// "Per client" or "`<p>` (`<p%>`%)".
    pub open_faucets: String,
    /// "Run traceability": Input network, Source file, Version, Format,
    /// Scenarios, Effective seed.
    pub traceability: Vec<(&'static str, String)>,
    /// Summary strip: Status, Scenarios, Successful, Execution failures,
    /// Success rate, Duration.
    pub strip: Vec<(&'static str, String)>,
    /// Failure records: Scenario, Stage, Code, Message.
    pub failures: Vec<[String; 4]>,
}

impl RunSummary {
    /// "`<n>` records" of the failure table.
    pub fn failure_count_text(&self) -> String {
        format!("{} records", self.failures.len())
    }
}

fn resolved<'a>(record: &'a RunRecord, key: &str) -> Option<&'a Json> {
    record
        .resolved_value(key)
        .map(|r| &r.value)
        .filter(|v| !matches!(v, Json::Null))
}

/// Summary of a run (values from the resolved options, then the requested
/// ones).
pub fn run_summary(record: &RunRecord) -> RunSummary {
    let bounded = match resolved(record, "boundedFlow") {
        Some(Json::Bool(b)) => Some(*b),
        _ => record.requested.bounded_flow,
    };
    let orifice = match resolved(record, "orificeMode") {
        Some(Json::String(t)) if t == "IDEAL" => "Ideal".to_string(),
        Some(Json::String(t)) if t == "COMMERCIAL" => "Commercial".to_string(),
        _ => record
            .requested
            .orifice_mode
            .map_or(MISSING.to_string(), |m| {
                crate::run::orifice_mode_label(m).to_string()
            }),
    };
    let uniform = !matches!(resolved(record, "uniformDemand"), Some(Json::Bool(false)));
    let target = if !uniform {
        "Per client".to_string()
    } else {
        resolved(record, "targetFlowLps")
            .and_then(Json::as_f64)
            .map_or(MISSING.to_string(), |q| format!("{} L/s", fixed(q, 2)))
    };
    let faucets = if !uniform {
        "Per client".to_string()
    } else {
        resolved(record, "openTapsProbability")
            .and_then(Json::as_f64)
            .map_or(MISSING.to_string(), |p| {
                format!("{} ({}%)", fixed(p, 2), fixed(p * 100.0, 0))
            })
    };
    let summary = record.statistics.as_ref().and_then(|s| s.get("summary"));
    let sv = |k: &str| summary.and_then(|s| get_f64(s, k));
    let count = |k: &str| sv(k).map_or(MISSING.to_string(), integer);
    RunSummary {
        title: run_label(record.run_number),
        configuration: record.configuration_mode.label(),
        bounded_flow: match bounded {
            Some(true) => "Enabled".into(),
            Some(false) => "Disabled".into(),
            None => MISSING.into(),
        },
        orifice_model: orifice,
        target_flow: target,
        open_faucets: faucets,
        traceability: vec![
            ("Input network", record.network_name.clone()),
            ("Source file", record.revision.clone()),
            ("Version", format!("v{}", record.revision_version)),
            ("Format", record.source_format.as_str().to_string()),
            (
                "Scenarios",
                record
                    .scenario_count()
                    .map_or(MISSING.into(), |c| c.to_string()),
            ),
            (
                "Effective seed",
                record
                    .effective_seed()
                    .map_or(MISSING.into(), |c| c.to_string()),
            ),
        ],
        strip: vec![
            ("Status", status_label(record.status).to_string()),
            ("Scenarios", count("scenarioCount")),
            ("Successful", count("successfulScenarios")),
            ("Execution failures", count("failedScenarios")),
            (
                "Success rate",
                sv("successRate").map_or(MISSING.to_string(), ratio_percent),
            ),
            (
                "Duration",
                sv("durationSeconds").map_or(MISSING.to_string(), |d| format!("{} s", adaptive(d))),
            ),
        ],
        failures: record
            .failures
            .iter()
            .map(|f| {
                [
                    f.scenario_index.to_string(),
                    f.stage.clone(),
                    f.code.clone(),
                    f.message.clone(),
                ]
            })
            .collect(),
    }
}

/// Execution details of a run (F41).
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionDetails {
    /// "Simulation #`<n>`".
    pub title: String,
    /// Input data: Network, File, File version, Format.
    pub input: Vec<(&'static str, String)>,
    /// Execution: Configuration, Status, Scenarios, Effective seed, Bounded
    /// flow, Orifice model.
    pub execution: Vec<(&'static str, String)>,
    /// Requested parameters (Parameter, Value).
    pub requested: Vec<(String, String)>,
    /// Text when nothing was requested.
    pub requested_empty: Option<&'static str>,
    /// Resolved parameters (Parameter, Value, Source).
    pub resolved: Vec<(String, String, &'static str)>,
    /// Text when no resolved set exists.
    pub resolved_empty: Option<&'static str>,
    /// "Pipe state used by this simulation · `<n>` pipes".
    pub pipe_state_title: String,
    /// Arc ID and "OPEN" / "CLOSED".
    pub pipe_states: Vec<(String, &'static str)>,
    /// Error message.
    pub error: Option<String>,
}

fn resolved_text(key: &str, v: &Json, unit: DisplayUnit) -> String {
    let length = |x: f64| format!("{} {}", fixed(unit.from_meters(x), 2), unit.symbol());
    match (key, v) {
        ("requiredPressureM" | "commercialPipeLengthM", Json::Number(x)) => length(*x),
        ("terminalRequiredPressureM", Json::Object(m)) => compact(&Json::Object(
            m.iter()
                .map(|(k, x)| {
                    (
                        k.clone(),
                        x.as_f64().map_or(Json::Null, |x| n(unit.from_meters(x))),
                    )
                })
                .collect(),
        )),
        _ => scalar_text(v).unwrap_or_else(|| MISSING.to_string()),
    }
}

/// Details of a run in the display unit.
pub fn execution_details(record: &RunRecord, unit: DisplayUnit) -> ExecutionDetails {
    let s = run_summary(record);
    let requested = record.requested.rows();
    ExecutionDetails {
        title: run_label(record.run_number),
        input: vec![
            ("Network", record.network_name.clone()),
            ("File", record.revision.clone()),
            ("File version", format!("v{}", record.revision_version)),
            ("Format", record.source_format.as_str().to_string()),
        ],
        execution: vec![
            ("Configuration", record.configuration_mode.label().to_string()),
            ("Status", status_label(record.status).to_string()),
            ("Scenarios", record.scenario_count().map_or(MISSING.into(), |c| c.to_string())),
            ("Effective seed", record.effective_seed().map_or(MISSING.into(), |c| c.to_string())),
            ("Bounded flow", s.bounded_flow.clone()),
            ("Orifice model", s.orifice_model.clone()),
        ],
        requested_empty: requested
            .is_empty()
            .then_some("No runtime overrides were submitted. Project defaults were used."),
        requested,
        resolved: record
            .resolved
            .iter()
            .map(|r| (r.key.clone(), resolved_text(&r.key, &r.value, unit), r.source.label()))
            .collect(),
        resolved_empty: record.resolved.is_empty().then_some(
            "No resolved parameter set is available because this execution did not complete successfully.",
        ),
        pipe_state_title: format!(
            "Pipe state used by this simulation · {} pipes",
            record.edge_states.len()
        ),
        pipe_states: record
            .edge_states
            .iter()
            .map(|(id, open)| (id.clone(), if *open { "OPEN" } else { "CLOSED" }))
            .collect(),
        error: record.error.as_ref().map(|e| e.message.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revision::SourceFormat;
    use crate::run::{ConfigurationMode, DemandGroup, RequestedOptions, TerminalDemand};

    fn record(stats: &str) -> RunRecord {
        RunRecord {
            run_number: 3,
            status: RunStatus::Completed,
            configuration_mode: ConfigurationMode::Default,
            revision: "Toy_v1.dsg".into(),
            revision_version: 1,
            source_format: SourceFormat::Dsg,
            network_name: "Toy_v1".into(),
            created_at: String::new(),
            started_at: None,
            completed_at: None,
            requested: RequestedOptions::default(),
            resolved: Vec::new(),
            edge_states: vec![("A1".into(), true), ("A2".into(), false)],
            demand: vec![
                TerminalDemand {
                    terminal_id: "P1".into(),
                    client_count: 1,
                    groups: vec![DemandGroup {
                        open_probability: 0.28,
                        target_flow_lps: 0.12,
                        client_count: 1,
                    }],
                },
                TerminalDemand {
                    terminal_id: "P10".into(),
                    client_count: 3,
                    groups: vec![
                        DemandGroup {
                            open_probability: 0.28,
                            target_flow_lps: 0.12,
                            client_count: 2,
                        },
                        DemandGroup {
                            open_probability: 0.5,
                            target_flow_lps: 0.12,
                            client_count: 1,
                        },
                    ],
                },
            ],
            statistics: Json::parse(stats).ok(),
            failures: Vec::new(),
            error: None,
        }
    }

    const STATS: &str = r#"{"units": {"sourceFlow": "L/s", "pipeVelocity": "m/s", "terminalPressure": "m"},
      "summary": {"scenarioCount": 10, "successfulScenarios": 9, "failedScenarios": 1, "successRate": 0.9, "durationSeconds": 0.0123},
      "statistics": {
        "terminalSatisfaction": [
          {"terminalId": "P10", "minSat": 0.5, "moySat": 0.9, "maxSat": 1.2, "cVarSat5": null, "cVarSat20": 0.6, "stdSat": 0.1, "cvSat": 0.111, "activeScenarioCount": 1234, "waterShortfallCount": 2},
          {"terminalId": "P2", "minSat": null, "moySat": null, "maxSat": null, "cVarSat5": null, "cVarSat20": null, "stdSat": null, "cvSat": null, "activeScenarioCount": 0, "waterShortfallCount": 0},
          {"terminalId": "P1", "minSat": 0.95, "moySat": 1, "maxSat": 1, "cVarSat5": 0.95, "cVarSat20": 0.97, "stdSat": 0, "cvSat": 0, "activeScenarioCount": 5, "waterShortfallCount": 0}
        ],
        "sourceFlow": [{"sourceId": "Source", "minSt": 0.1, "moySt": 0.2, "maxSt": 0.3, "cVarSt5": 0.1, "cVarSt95": 0.3}],
        "pipeVelocity": [{"edgeId": "A1", "startNodeId": "Source", "endNodeId": "A", "minVelocityMps": 0.1, "meanVelocityMps": 0.5, "maxVelocityMps": 1}],
        "terminalPressure": [{"terminalId": "P1", "minPressureM": 1, "meanPressureM": 2, "maxPressureM": 3}],
        "transitPressure": [{"transitNodeId": "A", "minPressureM": 10, "meanPressureM": 20, "maxPressureM": 30}]
      }}"#;

    #[test]
    fn terminal_satisfaction_table() {
        let r = record(STATS);
        let t = result_table(&r, TableKind::TerminalSatisfaction, DisplayUnit::Meter)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(t.columns[1].header(), "Minimum (%)");
        assert_eq!(t.columns[4].header(), "CVaR 5% (%)");
        assert_eq!(t.columns[7].header(), "Target flow / client (L/s)");
        assert_eq!(t.visible_columns(false).len(), 6);
        assert_eq!(t.visible_columns(true).len(), 13);
        let row = &t.rows[0];
        assert_eq!(row[0].role, Some(NodeRole::Terminal));
        assert_eq!(row[1].text, "50.00%");
        assert_eq!(row[3].text, "120.00%");
        assert_eq!(row[4].text, "—");
        assert_eq!(row[6].text, "3");
        assert_eq!(row[7].text, "2 groups");
        assert_eq!(row[9].text, "1,234");
        assert_eq!(row[12].text, "0.11");
        assert_eq!(t.rows[2][7].text, "0.12");
        assert_eq!(t.rows[1][6].text, "—");
        assert_eq!(t.row_count_text(), "3 rows");
    }

    #[test]
    fn sorting_rules() {
        let r = record(STATS);
        let t = result_table(&r, TableKind::TerminalSatisfaction, DisplayUnit::Meter)
            .unwrap_or_else(|| unreachable!());
        // natural order of IDs: P1, P2, P10
        let asc = next_sort(None, 0);
        assert_eq!(t.sorted_rows(asc, false), vec![2, 1, 0]);
        let desc = next_sort(asc, 0);
        assert_eq!(sort_indicator(desc, 0), "↓");
        assert_eq!(t.sorted_rows(desc, false), vec![0, 1, 2]);
        assert_eq!(next_sort(desc, 0), None);
        // numbers by value, empty values last in both directions
        let min_asc = next_sort(None, 1);
        assert_eq!(t.sorted_rows(min_asc, false), vec![0, 2, 1]);
        assert_eq!(t.sorted_rows(next_sort(min_asc, 1), false), vec![2, 0, 1]);
        // maxSat 1.2, null, 1: empty last
        assert_eq!(t.sorted_rows(next_sort(None, 3), false), vec![2, 0, 1]);
        // a hidden column does not sort
        assert_eq!(t.sorted_rows(next_sort(None, 9), false), vec![0, 1, 2]);
        assert_eq!(sort_indicator(None, 1), "↕");
    }

    #[test]
    fn units_and_pressure_table() {
        let r = record(STATS);
        let p = result_table(&r, TableKind::Pressure, DisplayUnit::Feet)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(p.columns[2].header(), "Minimum (ft)");
        assert_eq!(p.rows.len(), 2);
        assert_eq!(p.rows[0][0].text, "P1");
        assert_eq!(p.rows[0][1].text, "Terminal");
        assert_eq!(p.rows[1][1].text, "Junction");
        assert_eq!(p.rows[1][0].role, Some(NodeRole::Junction));
        assert_eq!(p.rows[0][2].text, "3.28");
        assert_eq!(p.rows[0][2].sort, SortValue::Number(1.0));
        let v = result_table(&r, TableKind::PipeVelocity, DisplayUnit::Inches)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(v.columns[3].header(), "Minimum (in/s)");
        assert_eq!(v.rows[0][1].role, Some(NodeRole::Source));
        assert_eq!(v.rows[0][2].role, Some(NodeRole::Junction));
        let s = result_table(&r, TableKind::SourceFlow, DisplayUnit::Feet)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(s.columns[1].header(), "Minimum (L/s)");
        assert_eq!(s.rows[0][1].text, "0.10");
        assert_eq!(result_tables(&r, DisplayUnit::Meter).len(), 4);
        let mut none = record(STATS);
        none.statistics = None;
        assert!(result_tables(&none, DisplayUnit::Meter).is_empty());
    }

    #[test]
    fn summary_and_details() {
        let mut r = record(STATS);
        r.resolved = vec![
            crate::run::ResolvedValue {
                key: "targetFlowLps".into(),
                value: Json::Number(0.12),
                source: crate::run::ParameterSource::ClientDemandTable,
            },
            crate::run::ResolvedValue {
                key: "openTapsProbability".into(),
                value: Json::Number(0.28),
                source: crate::run::ParameterSource::ClientDemandTable,
            },
            crate::run::ResolvedValue {
                key: "requiredPressureM".into(),
                value: Json::Number(2.0),
                source: crate::run::ParameterSource::ProjectDefault,
            },
            crate::run::ResolvedValue {
                key: "boundedFlow".into(),
                value: Json::Bool(false),
                source: crate::run::ParameterSource::ProjectDefault,
            },
        ];
        let s = run_summary(&r);
        assert_eq!(s.title, "Simulation #3");
        assert_eq!(s.target_flow, "0.12 L/s");
        assert_eq!(s.open_faucets, "0.28 (28%)");
        assert_eq!(s.bounded_flow, "Disabled");
        assert_eq!(s.orifice_model, "—");
        assert_eq!(s.strip[4], ("Success rate", "90.00%".to_string()));
        assert_eq!(s.strip[5], ("Duration", "0.01 s".to_string()));
        let d = execution_details(&r, DisplayUnit::Feet);
        assert_eq!(d.resolved[2].1, "6.56 ft");
        assert_eq!(d.resolved[0].2, "Client demand table");
        assert_eq!(
            d.pipe_state_title,
            "Pipe state used by this simulation · 2 pipes"
        );
        assert_eq!(d.pipe_states[1], ("A2".to_string(), "CLOSED"));
        assert!(d.requested_empty.is_some());
    }
}
