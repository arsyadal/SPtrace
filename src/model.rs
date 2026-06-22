use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Clone)]
pub struct ProcedureTrace {
    pub name: Option<String>,
    pub parameters: Vec<Parameter>,
    pub dependencies: Vec<Dependency>,
    pub temp_tables: Vec<String>,
    pub risks: Vec<RiskFinding>,
    pub statements: Vec<StatementSummary>,
    pub metrics: TraceMetrics,
}

impl Default for ProcedureTrace {
    fn default() -> Self {
        Self {
            name: None,
            parameters: Vec::new(),
            dependencies: Vec::new(),
            temp_tables: Vec::new(),
            risks: Vec::new(),
            statements: Vec::new(),
            metrics: TraceMetrics::default(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TraceMetrics {
    pub statement_count: usize,
    pub read_count: usize,
    pub write_count: usize,
    pub risk_level: Severity,
}

impl Default for TraceMetrics {
    fn default() -> Self {
        Self {
            statement_count: 0,
            read_count: 0,
            write_count: 0,
            risk_level: Severity::Low,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Parameter {
    pub name: String,
    pub data_type: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    pub object: String,
    pub operation: Operation,
    pub source: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    Read,
    Write,
    Execute,
    Unknown,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Operation::Read => "READ",
            Operation::Write => "WRITE",
            Operation::Execute => "EXECUTE",
            Operation::Unknown => "UNKNOWN",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct RiskFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    #[serde(alias = "low", alias = "LOW", alias = "Low")]
    Low,
    #[serde(alias = "medium", alias = "MEDIUM", alias = "Medium")]
    Medium,
    #[serde(alias = "high", alias = "HIGH", alias = "High")]
    High,
}

impl Severity {
    pub fn rank(&self) -> u8 {
        match self {
            Severity::High => 3,
            Severity::Medium => 2,
            Severity::Low => 1,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanIndexEntry {
    pub source_file: String,
    pub report_file: String,
    pub procedure: String,
    pub read_count: usize,
    pub write_count: usize,
    pub risk_level: Severity,
    pub risk_rules: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct StatementSummary {
    pub index: usize,
    pub kind: StatementKind,
    pub target: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub enum StatementKind {
    Select,
    Insert,
    Update,
    Delete,
    Merge,
    Execute,
    Transaction,
    Unknown,
}

impl fmt::Display for StatementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            StatementKind::Select => "Select",
            StatementKind::Insert => "Insert",
            StatementKind::Update => "Update",
            StatementKind::Delete => "Delete",
            StatementKind::Merge => "Merge",
            StatementKind::Execute => "Execute",
            StatementKind::Transaction => "Transaction",
            StatementKind::Unknown => "Unknown",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub rules: std::collections::HashMap<String, RuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RuleConfig {
    Bool(bool),
    Severity(Severity),
}
