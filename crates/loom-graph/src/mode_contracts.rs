//! e-mode-contract — typed views over `loom:g/mode-contracts` and the
//! consistency oracle (loom-implementation §3.4): a contract is derived from
//! its component's core machine — no phantom events, no unreported states.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::graph::NamedGraph;
use crate::query::{QueryError, QueryExecutor};

/// Maintained query: contract surfaces, one row per (component, surface item).
pub const MODE_CONTRACTS_QUERY: &str = include_str!("../queries/mode_contracts.sparql");
/// Maintained query: machine alphabets and state ranges.
pub const MACHINES_QUERY: &str = include_str!("../queries/machines.sparql");

#[derive(Debug, Clone, Deserialize)]
pub struct ModeContractRow {
    pub component: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub descriptor_state: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineRow {
    pub component: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

/// A component's mode-contract, folded from rows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ModeContract {
    pub component: String,
    pub event_surface: BTreeSet<String>,
    pub descriptor_states: BTreeSet<String>,
    pub lifecycle: BTreeSet<String>,
}

/// A component's machine surface, folded from rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MachineSurface {
    pub component: String,
    pub events: BTreeSet<String>,
    pub states: BTreeSet<String>,
}

pub fn contract_views<T: QueryExecutor>(
    executor: &T,
    graph: &NamedGraph,
) -> Result<Vec<ModeContract>, QueryError> {
    let rows: Vec<ModeContractRow> = executor.fetch_rows(graph, MODE_CONTRACTS_QUERY)?;
    let mut folded: BTreeMap<String, ModeContract> = BTreeMap::new();
    for row in rows {
        let entry = folded.entry(row.component.clone()).or_default();
        entry.component = row.component;
        if let Some(e) = row.event {
            entry.event_surface.insert(e);
        }
        if let Some(s) = row.descriptor_state {
            entry.descriptor_states.insert(s);
        }
        if let Some(l) = row.lifecycle {
            entry.lifecycle.insert(l);
        }
    }
    Ok(folded.into_values().collect())
}

pub fn machine_surfaces<T: QueryExecutor>(
    executor: &T,
    graph: &NamedGraph,
) -> Result<Vec<MachineSurface>, QueryError> {
    let rows: Vec<MachineRow> = executor.fetch_rows(graph, MACHINES_QUERY)?;
    let mut folded: BTreeMap<String, MachineSurface> = BTreeMap::new();
    for row in rows {
        let entry = folded.entry(row.component.clone()).or_default();
        entry.component = row.component;
        if let Some(e) = row.event {
            entry.events.insert(e);
        }
        if let Some(s) = row.state {
            entry.states.insert(s);
        }
    }
    Ok(folded.into_values().collect())
}

/// One component's consistency verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsistencyReport {
    pub component: String,
    /// Events on the contract surface that the machine never accepts.
    pub phantom_events: Vec<String>,
    /// Machine states the descriptor shape does not report.
    pub unreported_states: Vec<String>,
}

impl ConsistencyReport {
    pub fn consistent(&self) -> bool {
        self.phantom_events.is_empty() && self.unreported_states.is_empty()
    }
}

/// The §3.4 consistency oracle — pure judge over the two fetched surfaces.
pub fn judge_contract_consistency(
    contract: &ModeContract,
    machine: &MachineSurface,
) -> ConsistencyReport {
    ConsistencyReport {
        component: contract.component.clone(),
        phantom_events: contract
            .event_surface
            .difference(&machine.events)
            .cloned()
            .collect(),
        unreported_states: machine
            .states
            .difference(&contract.descriptor_states)
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(events: &[&str], states: &[&str]) -> ModeContract {
        ModeContract {
            component: "single-select".into(),
            event_surface: events.iter().map(|s| s.to_string()).collect(),
            descriptor_states: states.iter().map(|s| s.to_string()).collect(),
            lifecycle: ["mount", "unmount"].iter().map(|s| s.to_string()).collect(),
        }
    }

    fn machine(events: &[&str], states: &[&str]) -> MachineSurface {
        MachineSurface {
            component: "single-select".into(),
            events: events.iter().map(|s| s.to_string()).collect(),
            states: states.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn derived_contract_is_consistent() {
        let report = judge_contract_consistency(
            &contract(&["open", "close"], &["closed", "open"]),
            &machine(&["open", "close"], &["closed", "open"]),
        );
        assert!(report.consistent());
    }

    #[test]
    fn phantom_event_is_reported() {
        let report = judge_contract_consistency(
            &contract(&["open", "explode"], &["closed", "open"]),
            &machine(&["open"], &["closed", "open"]),
        );
        assert_eq!(report.phantom_events, vec!["explode".to_string()]);
        assert!(!report.consistent());
    }

    #[test]
    fn unreported_state_is_reported() {
        let report = judge_contract_consistency(
            &contract(&["open"], &["closed"]),
            &machine(&["open"], &["closed", "open"]),
        );
        assert_eq!(report.unreported_states, vec!["open".to_string()]);
        assert!(!report.consistent());
    }
}
