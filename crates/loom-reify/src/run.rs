//! e-reification-run — run lifecycle over the canonical seam.
//!
//! The run is a PROV-O Activity in `loom:g/runs/<id>`; every check verdict is
//! appended as produced, and the report is a projection over that graph —
//! never assembled from memory (p-run-graph-append).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A reification run over one canonical UIIntent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReificationRun {
    pub run_id: String,
    pub intent_id: String,
    pub status: RunStatus,
    pub verdicts: Vec<CheckVerdict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
}

/// One oracle verdict, appended to the run graph as it is produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckVerdict {
    pub check: String,
    pub verdict: Verdict,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    Incoherence,
}

#[derive(Error, Debug)]
pub enum RunError {
    #[error("run graph write failed: {0}")]
    Graph(#[from] loom_graph::BundleError),
}

impl ReificationRun {
    pub fn start(run_id: impl Into<String>, intent_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            intent_id: intent_id.into(),
            status: RunStatus::Running,
            verdicts: Vec::new(),
        }
    }

    /// Append a verdict as produced. Any INCOHERENCE fails the run.
    pub fn record(&mut self, verdict: CheckVerdict) {
        if verdict.verdict != Verdict::Pass {
            self.status = RunStatus::Failed;
        }
        self.verdicts.push(verdict);
    }

    pub fn complete(mut self) -> Self {
        if self.status == RunStatus::Running {
            self.status = RunStatus::Completed;
        }
        self
    }
}
