//! Errors with the python-core codes (spec section 6).

use crate::json::Value;

/// One input or option problem (`ValidationIssue`, `sim/exceptions.py`).
#[derive(Clone, Debug, PartialEq)]
pub struct ValidationIssue {
    /// Machine code, for example `UNKNOWN_PIPE_DIAMETER`.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// `node`, `pipe`, `client`, `borne`, `diameter`, `parameter`, `table`
    /// or `option`.
    pub entity_type: Option<String>,
    /// Identifier of the faulty entity.
    pub entity_id: Option<String>,
    /// Faulty field (`lon1`, `proba`, ...).
    pub field: Option<String>,
    /// Extra values.
    pub details: Vec<(String, Value)>,
}

impl ValidationIssue {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> ValidationIssue {
        ValidationIssue {
            code,
            message: message.into(),
            entity_type: None,
            entity_id: None,
            field: None,
            details: Vec::new(),
        }
    }

    pub(crate) fn entity(mut self, kind: &str, id: impl Into<String>) -> ValidationIssue {
        self.entity_type = Some(kind.to_string());
        self.entity_id = Some(id.into());
        self
    }

    pub(crate) fn field(mut self, field: &str) -> ValidationIssue {
        self.field = Some(field.to_string());
        self
    }

    pub(crate) fn detail(mut self, key: &str, value: impl Into<Value>) -> ValidationIssue {
        self.details.push((key.to_string(), value.into()));
        self
    }

    /// JSON form (`ValidationIssue.to_dict`: populated fields only).
    pub fn to_value(&self) -> Value {
        let mut m = vec![
            ("code".to_string(), Value::from(self.code)),
            ("message".to_string(), Value::from(self.message.as_str())),
        ];
        if let Some(v) = &self.entity_type {
            m.push(("entityType".into(), Value::from(v.as_str())));
        }
        if let Some(v) = &self.entity_id {
            m.push(("entityId".into(), Value::from(v.as_str())));
        }
        if let Some(v) = &self.field {
            m.push(("field".into(), Value::from(v.as_str())));
        }
        if !self.details.is_empty() {
            m.push(("details".into(), Value::Map(self.details.clone())));
        }
        Value::Map(m)
    }
}

/// An error that stops a simulation run.
#[derive(Clone, Debug, PartialEq)]
pub enum SimError {
    /// `SIMULATION_INPUT_INVALID`: input or option issues, all collected.
    Validation(Vec<ValidationIssue>),
    /// `PREPROCESSING_FAILED`: a hydraulic table cannot be built (python
    /// raises a plain `ValueError`/`RuntimeError` here, without a code).
    Preprocessing(String),
    /// `SCENARIO_GENERATION_FAILED`.
    ScenarioGeneration {
        /// Message.
        message: String,
        /// Extra values.
        details: Vec<(String, Value)>,
    },
    /// `SCENARIO_PREPARATION_FAILED` raised before the batch (problem
    /// builder set-up).
    ScenarioPreparation {
        /// Message.
        message: String,
        /// Extra values.
        details: Vec<(String, Value)>,
    },
    /// `STATISTICS_PROCESSING_FAILED`, for example when no scenario
    /// succeeded.
    StatisticsProcessing {
        /// Message.
        message: String,
        /// Extra values.
        details: Vec<(String, Value)>,
    },
    /// `SIMULATION_CANCELLED`: the progress callback asked to stop.
    Cancelled(String),
}

impl SimError {
    /// python-core error code.
    pub fn code(&self) -> &'static str {
        match self {
            SimError::Validation(_) => "SIMULATION_INPUT_INVALID",
            SimError::Preprocessing(_) => "PREPROCESSING_FAILED",
            SimError::ScenarioGeneration { .. } => "SCENARIO_GENERATION_FAILED",
            SimError::ScenarioPreparation { .. } => "SCENARIO_PREPARATION_FAILED",
            SimError::StatisticsProcessing { .. } => "STATISTICS_PROCESSING_FAILED",
            SimError::Cancelled(_) => "SIMULATION_CANCELLED",
        }
    }

    /// Issues of a validation error (empty for other errors).
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            SimError::Validation(v) => v,
            _ => &[],
        }
    }

    /// JSON form `{code, message, details?|issues?}` (`to_dict`).
    pub fn to_value(&self) -> Value {
        let mut m = vec![
            ("code".to_string(), Value::from(self.code())),
            ("message".to_string(), Value::from(self.to_string())),
        ];
        match self {
            SimError::Validation(issues) => {
                m.push((
                    "issues".into(),
                    Value::List(issues.iter().map(ValidationIssue::to_value).collect()),
                ));
            }
            SimError::ScenarioGeneration { details, .. }
            | SimError::ScenarioPreparation { details, .. }
            | SimError::StatisticsProcessing { details, .. }
                if !details.is_empty() =>
            {
                m.push(("details".into(), Value::Map(details.clone())));
            }
            _ => {}
        }
        Value::Map(m)
    }
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimError::Validation(issues) => {
                write!(
                    f,
                    "Simulation input validation failed with {} issue(s)",
                    issues.len()
                )?;
                for i in issues {
                    write!(f, "; {}: {}", i.code, i.message)?;
                }
                Ok(())
            }
            SimError::Preprocessing(m) | SimError::Cancelled(m) => f.write_str(m),
            SimError::ScenarioGeneration { message, .. }
            | SimError::ScenarioPreparation { message, .. }
            | SimError::StatisticsProcessing { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for SimError {}

/// Failure of one scenario before or inside the solver.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StageError {
    pub(crate) message: String,
    pub(crate) details: Vec<(String, Value)>,
}

impl StageError {
    pub(crate) fn new(message: impl Into<String>) -> StageError {
        StageError {
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub(crate) fn detail(mut self, key: &str, value: impl Into<Value>) -> StageError {
        self.details.push((key.to_string(), value.into()));
        self
    }
}
