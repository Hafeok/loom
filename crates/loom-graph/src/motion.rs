//! e-motion-entry — typed views over `loom:g/motion` and the §7.6 motion check.
//!
//! Machine transitions reference catalog entries by name only; an entry
//! without a reduced-motion fallback fails validation (loom-spec §5,
//! WCAG 2.3.3). Queries fetch, this module's judges decide.

use serde::{Deserialize, Serialize};

use crate::graph::NamedGraph;
use crate::query::{QueryError, QueryExecutor};

/// Maintained query: every motion entry with its fields.
pub const MOTION_QUERY: &str = include_str!("../queries/motion.sparql");

/// One motion catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MotionEntryRow {
    pub entry: String,
    pub name: String,
    pub communicates: String,
    pub duration: String,
    pub easing: String,
    pub reduced_motion_fallback: String,
}

/// Fetch the motion entries for a graph.
pub fn motion_rows<T: QueryExecutor>(
    executor: &T,
    graph: &NamedGraph,
) -> Result<Vec<MotionEntryRow>, QueryError> {
    executor.fetch_rows(graph, MOTION_QUERY)
}

/// WCAG 2.3.3: a reduced-motion fallback must exist and be non-empty
/// (whitespace-only is treated as empty).
pub fn judge_reduced_motion_fallback(fallback: &str) -> bool {
    !fallback.trim().is_empty()
}

/// A machine transition that claims to carry motion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MotionReference {
    pub component: String,
    /// The referenced catalog entry name; `None` means the transition
    /// explicitly declares no motion — always legal.
    pub carries_motion: Option<String>,
}

/// A transition whose named motion has no catalog entry — incoherence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DanglingMotion {
    pub component: String,
    pub referenced_motion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MotionCheckReport {
    pub passes: bool,
    pub dangling: Vec<DanglingMotion>,
}

/// The §7.6 motion oracle: every transition carrying motion names an existing
/// catalog entry or declares none.
pub fn judge_motion_references(
    entry_names: &[String],
    references: &[MotionReference],
) -> MotionCheckReport {
    let dangling: Vec<DanglingMotion> = references
        .iter()
        .filter_map(|r| {
            let name = r.carries_motion.as_ref()?;
            if entry_names.iter().any(|e| e == name) {
                None
            } else {
                Some(DanglingMotion {
                    component: r.component.clone(),
                    referenced_motion: name.clone(),
                })
            }
        })
        .collect();
    MotionCheckReport {
        passes: dangling.is_empty(),
        dangling,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_fallback_fails_wcag_2_3_3() {
        assert!(judge_reduced_motion_fallback("instant"));
        assert!(!judge_reduced_motion_fallback(""));
        assert!(!judge_reduced_motion_fallback("   "));
    }

    #[test]
    fn dangling_motion_reference_is_reported() {
        let report = judge_motion_references(
            &["fade-in".into()],
            &[MotionReference {
                component: "Tooltip".into(),
                carries_motion: Some("zoom-out".into()),
            }],
        );
        assert!(!report.passes);
        assert_eq!(report.dangling[0].referenced_motion, "zoom-out");
    }

    #[test]
    fn declared_none_is_legal() {
        let report = judge_motion_references(
            &[],
            &[MotionReference {
                component: "Checkbox".into(),
                carries_motion: None,
            }],
        );
        assert!(report.passes);
    }
}
