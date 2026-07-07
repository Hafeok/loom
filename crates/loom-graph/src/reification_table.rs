//! Reification Row — typed views, completeness query, and leaf-completeness oracle.
//!
//! Implements pat-typed-row-view for `e-reification-row`:
//! - `queries/completeness_construct.sparql` → typed rows via SPARQL CONSTRUCT
//! - `Row` struct → `ReificationRow` via `From`
//! - §7.1 leaf-completeness oracle: pure Rust judge over queried facts

use std::fmt;

use crate::graph::NamedGraph;
use crate::query::QueryExecutor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The §2.4 completeness CONSTRUCT query file.
/// Returns reified/unreifiable partition joined to intent's aios-in-context.
pub const COMPLETENESS_CONSTRUCT_QUERY: &str = include_str!("../queries/completeness_construct.sparql");

/// The §7.1 leaf-completeness oracle judge function.
/// Pure Rust over queried facts — never computed inside SPARQL.

// ─── Row definition ───────────────────────────────────────────────────────────

/// A reification row from the SPARQL CONSTRUCT.
/// Context objects (platform, form-factor, interaction-class) appear as nodes, not strings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ReificationRow {
    pub aio_kind: String,
    pub platform: NodeRef,
    pub form_factor: NodeRef,
    pub interaction_class: NodeRef,
    pub cio: String,
}

impl fmt::Display for ReificationRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ReificationRow({} @ {}/{} (context {})) → {}",
            self.aio_kind, self.platform, self.form_factor, self.interaction_class, self.cio
        )
    }
}

/// A context object node reference — not a string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeRef {
    /// The RDF node identifier (IRI or blank node).
    pub node: String,
    /// Display label if available.
    pub label: Option<String>,
}

impl fmt::Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.label {
            Some(label) => write!(f, "{} ({})", label, self.node),
            None => write!(f, "{}", self.node),
        }
    }
}

// ─── View over SPARQL results ────────────────────────────────────────────────

/// Build a `ReificationRow` from raw SPARQL query results.
/// This is the `From` implementation required by pat-typed-row-view.
pub fn rows_from_results<T: QueryExecutor>(
    executor: &T,
    named_graph: NamedGraph,
) -> Result<Vec<ReificationRow>, ReificationError> {
    let rows: Vec<RowRaw> = executor
        .fetch_rows(&named_graph, COMPLETENESS_CONSTRUCT_QUERY)?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, ReificationError>>()
}

/// Raw row from SPARQL — intermediate representation before typed view.
#[derive(Deserialize)]
struct RowRaw {
    aio_kind: String,
    platform_node: String,
    platform_label: Option<String>,
    form_factor_node: String,
    form_factor_label: Option<String>,
    interaction_class_node: String,
    interaction_class_label: Option<String>,
    cio: String,
}

impl TryFrom<RowRaw> for ReificationRow {
    type Error = ReificationError;

    fn try_from(raw: RowRaw) -> Result<Self, Self::Error> {
        if raw.platform_node.is_empty() {
            return Err(ReificationError::MissingField("platform".into()));
        }
        if raw.form_factor_node.is_empty() {
            return Err(ReificationError::MissingField("form_factor".into()));
        }
        if raw.interaction_class_node.is_empty() {
            return Err(ReificationError::MissingField("interaction_class".into()));
        }

        Ok(Self {
            aio_kind: raw.aio_kind,
            platform: NodeRef {
                node: raw.platform_node,
                label: raw.platform_label,
            },
            form_factor: NodeRef {
                node: raw.form_factor_node,
                label: raw.form_factor_label,
            },
            interaction_class: NodeRef {
                node: raw.interaction_class_node,
                label: raw.interaction_class_label,
            },
            cio: raw.cio,
        })
    }
}

// ─── Completeness report (§2.4 / §7.1) ───────────────────────────────────────

/// The partition produced by the §2.4 completeness CONSTRUCT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletenessReport {
    /// Aios that resolved to a component via reification rows.
    pub reified: Vec<AioAssignment>,
    /// Aios that could not be resolved, with the core reason.
    pub unreifiable: Vec<UnreifiableAssignment>,
}

/// An aio that was successfully bound to a CIO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AioAssignment {
    pub aio_kind: String,
    pub platform: NodeRef,
    pub form_factor: NodeRef,
    pub interaction_class: NodeRef,
    pub cio: String,
}

impl From<ReificationRow> for AioAssignment {
    fn from(row: ReificationRow) -> Self {
        Self {
            aio_kind: row.aio_kind,
            platform: row.platform,
            form_factor: row.form_factor,
            interaction_class: row.interaction_class,
            cio: row.cio,
        }
    }
}

/// An aio that could not be resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnreifiableAssignment {
    pub aio_kind: String,
    pub platform: NodeRef,
    pub form_factor: NodeRef,
    pub interaction_class: NodeRef,
    pub reason: UnreifiableReason,
}

/// Core unreifiable reasons from loom-spec §2.4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnreifiableReason {
    /// The intent references an AIO kind that has no binding for the given context.
    NoBindingForContext,
    /// The intent references an AIO kind not declared in the token graph.
    UnknownAioKind,
    /// The platform lacks the capability required for this AIO × context.
    CapabilityAbsent,
}

impl fmt::Display for UnreifiableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBindingForContext => write!(f, "no-binding-for-context"),
            Self::UnknownAioKind => write!(f, "unknown-aio-kind"),
            Self::CapabilityAbsent => write!(f, "capability-absent"),
        }
    }
}

/// The §7.1 leaf-completeness oracle: judge function.
///
/// Takes an intent's aios-in-context and the reification rows from the catalog,
/// and returns the completeness report with partition and reasons.
pub fn judge_leaf_completeness(
    intent_aios: &[(String, NodeRef, NodeRef, NodeRef)],
    reified_rows: &[ReificationRow],
    known_aio_kinds: &[String],
    platform_capabilities: &[(NodeRef, Vec<String>)],
) -> CompletenessReport {
    let mut reified = Vec::new();
    let mut unreifiable = Vec::new();

    for (aio_kind, platform, form_factor, interaction_class) in intent_aios {
        // Check if the aio_kind is known in the token graph.
        if !known_aio_kinds.iter().any(|k| k == aio_kind) {
            unreifiable.push(UnreifiableAssignment {
                aio_kind: aio_kind.clone(),
                platform: platform.clone(),
                form_factor: form_factor.clone(),
                interaction_class: interaction_class.clone(),
                reason: UnreifiableReason::UnknownAioKind,
            });
            continue;
        }

        // Check for exact match in reification rows.
        let matched = reified_rows.iter().find(|r| {
            r.aio_kind == *aio_kind
                && r.platform.node == platform.node
                && r.form_factor.node == form_factor.node
                && r.interaction_class.node == interaction_class.node
        });

        match matched {
            Some(row) => {
                reified.push(AioAssignment {
                    aio_kind: row.aio_kind.clone(),
                    platform: row.platform.clone(),
                    form_factor: row.form_factor.clone(),
                    interaction_class: row.interaction_class.clone(),
                    cio: row.cio.clone(),
                });
            }
            None => {
                // A kind is capability-gated when some platform declares it.
                // Gated + absent on this platform → the platform cannot host it;
                // otherwise the catalog simply lacks a binding for this context.
                let kind_key = aio_kind.to_lowercase();
                let capability_gated = platform_capabilities
                    .iter()
                    .any(|(_, caps)| caps.contains(&kind_key));
                let platform_has_capability =
                    platform_capabilities.iter().any(|(cap_platform, caps)| {
                        cap_platform.node == platform.node && caps.contains(&kind_key)
                    });

                let reason = if capability_gated && !platform_has_capability {
                    UnreifiableReason::CapabilityAbsent
                } else {
                    UnreifiableReason::NoBindingForContext
                };

                unreifiable.push(UnreifiableAssignment {
                    aio_kind: aio_kind.clone(),
                    platform: platform.clone(),
                    form_factor: form_factor.clone(),
                    interaction_class: interaction_class.clone(),
                    reason,
                });
            }
        }
    }

    CompletenessReport { reified, unreifiable }
}

/// A duplicate (AIO-kind, platform, form-factor, interaction-class) key —
/// two rows matching the same pair is an ambiguity and fails validation (§3.1).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AmbiguousKey {
    pub aio_kind: String,
    pub platform: String,
    pub form_factor: String,
    pub interaction_class: String,
    pub cios: Vec<String>,
}

/// The exactly-once oracle: each AIO×context resolves to exactly one component.
pub fn judge_row_ambiguity(rows: &[ReificationRow]) -> Vec<AmbiguousKey> {
    let mut by_key: std::collections::BTreeMap<(String, String, String, String), Vec<String>> =
        std::collections::BTreeMap::new();
    for row in rows {
        by_key
            .entry((
                row.aio_kind.clone(),
                row.platform.node.clone(),
                row.form_factor.node.clone(),
                row.interaction_class.node.clone(),
            ))
            .or_default()
            .push(row.cio.clone());
    }
    by_key
        .into_iter()
        .filter(|(_, cios)| cios.len() > 1)
        .map(|((aio_kind, platform, form_factor, interaction_class), cios)| AmbiguousKey {
            aio_kind,
            platform,
            form_factor,
            interaction_class,
            cios,
        })
        .collect()
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ReificationError {
    #[error("Missing field: {0}")]
    MissingField(String),

    #[error(transparent)]
    Query(#[from] crate::query::QueryError),

    #[error("Duplicate reification row: {0}")]
    DuplicateRow(String),

    #[error("Incomplete reification row for AIO: {0}")]
    IncompleteRow(String),
}

// ─── Vector tests (§7.1 leaf-completeness oracle) ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, label: Option<&str>) -> NodeRef {
        NodeRef {
            node: format!("http://example.com/ns#{}", id),
            label: label.map(String::from),
        }
    }

    #[test]
    fn oracle_reifies_matching_rows() {
        let intent_aios = vec![
            (
                "modal".into(),
                make_node("desktop", Some("Desktop")),
                make_node("screen", Some("Screen")),
                make_node("keyboard", Some("Keyboard")),
            ),
            (
                "tooltip".into(),
                make_node("mobile", Some("Mobile")),
                make_node("touch", Some("Touch")),
                make_node("touch", Some("Touch")),
            ),
        ];

        let known_kinds = vec!["modal".into(), "tooltip".into(), "panel".into()];
        let capabilities = vec![];

        let rows = vec![
            ReificationRow {
                aio_kind: "modal".into(),
                platform: make_node("desktop", Some("Desktop")),
                form_factor: make_node("screen", Some("Screen")),
                interaction_class: make_node("keyboard", Some("Keyboard")),
                cio: "ModalDialog".into(),
            },
            ReificationRow {
                aio_kind: "tooltip".into(),
                platform: make_node("mobile", Some("Mobile")),
                form_factor: make_node("touch", Some("Touch")),
                interaction_class: make_node("touch", Some("Touch")),
                cio: "ToastNotification".into(),
            },
        ];

        let report = judge_leaf_completeness(&intent_aios, &rows, &known_kinds, &capabilities);

        assert_eq!(report.reified.len(), 2);
        assert_eq!(report.reified[0].cio, "ModalDialog");
        assert_eq!(report.reified[1].cio, "ToastNotification");
        assert!(report.unreifiable.is_empty());
    }

    #[test]
    fn oracle_rejects_unknown_aio_kind() {
        let intent_aios = vec![(
            "ghost".into(),
            make_node("desktop", Some("Desktop")),
            make_node("screen", Some("Screen")),
            make_node("keyboard", Some("Keyboard")),
        )];

        let known_kinds = vec!["modal".into()];
        let report = judge_leaf_completeness(&intent_aios, &[], &known_kinds, &[]);

        assert_eq!(report.reified.len(), 0);
        assert_eq!(report.unreifiable.len(), 1);
        assert_eq!(
            report.unreifiable[0].reason,
            UnreifiableReason::UnknownAioKind
        );
    }

    #[test]
    fn oracle_no_binding_for_context() {
        let intent_aios = vec![(
            "modal".into(),
            make_node("tablet", Some("Tablet")),
            make_node("screen", Some("Screen")),
            make_node("keyboard", Some("Keyboard")),
        )];

        let known_kinds = vec!["modal".into()];
        let capabilities = vec![];
        let rows = vec![ReificationRow {
            aio_kind: "modal".into(),
            platform: make_node("desktop", Some("Desktop")),
            form_factor: make_node("screen", Some("Screen")),
            interaction_class: make_node("keyboard", Some("Keyboard")),
            cio: "ModalDialog".into(),
        }];

        let report = judge_leaf_completeness(&intent_aios, &rows, &known_kinds, &capabilities);

        assert_eq!(report.reified.len(), 0);
        assert_eq!(report.unreifiable.len(), 1);
        assert_eq!(
            report.unreifiable[0].reason,
            UnreifiableReason::NoBindingForContext
        );
    }

    #[test]
    fn oracle_capability_absent() {
        let intent_aios = vec![(
            "haptic_feedback".into(),
            make_node("mobile", Some("Mobile")),
            make_node("touch", Some("Touch")),
            make_node("touch", Some("Touch")),
        )];

        let known_kinds = vec!["haptic_feedback".into()];
        // Desktop has haptic, mobile does not
        let capabilities = vec![(
            make_node("desktop", Some("Desktop")),
            vec!["haptic_feedback".to_string()],
        )];

        let report = judge_leaf_completeness(&intent_aios, &[], &known_kinds, &capabilities);

        assert_eq!(report.reified.len(), 0);
        assert_eq!(report.unreifiable.len(), 1);
        assert_eq!(
            report.unreifiable[0].reason,
            UnreifiableReason::CapabilityAbsent
        );
    }

    #[test]
    fn vector_positive_fixture() {
        // Positive fixture: all aios resolve
        let intent_aios = vec![
            (
                "button".into(),
                make_node("web", Some("Web")),
                make_node("screen", Some("Screen")),
                make_node("pointer", Some("Pointer")),
            ),
            (
                "switch".into(),
                make_node("web", Some("Web")),
                make_node("screen", Some("Screen")),
                make_node("pointer", Some("Pointer")),
            ),
        ];

        let known_kinds = vec!["button".into(), "switch".into()];
        let rows = vec![
            ReificationRow {
                aio_kind: "button".into(),
                platform: make_node("web", Some("Web")),
                form_factor: make_node("screen", Some("Screen")),
                interaction_class: make_node("pointer", Some("Pointer")),
                cio: "HtmlButton".into(),
            },
            ReificationRow {
                aio_kind: "switch".into(),
                platform: make_node("web", Some("Web")),
                form_factor: make_node("screen", Some("Screen")),
                interaction_class: make_node("pointer", Some("Pointer")),
                cio: "HtmlCheckbox".into(),
            },
        ];

        let report = judge_leaf_completeness(&intent_aios, &rows, &known_kinds, &[]);

        assert_eq!(report.reified.len(), 2);
        assert!(report.unreifiable.is_empty());
    }

    #[test]
    fn vector_negative_fixture() {
        // Negative fixture: partial resolution with known reasons
        let intent_aios = vec![
            (
                "modal".into(),
                make_node("web", Some("Web")),
                make_node("screen", Some("Screen")),
                make_node("pointer", Some("Pointer")),
            ),
            (
                "nonexistent".into(),
                make_node("web", Some("Web")),
                make_node("screen", Some("Screen")),
                make_node("pointer", Some("Pointer")),
            ),
            (
                "haptic".into(),
                make_node("web", Some("Web")),
                make_node("screen", Some("Screen")),
                make_node("pointer", Some("Pointer")),
            ),
        ];

        let known_kinds = vec!["modal".into(), "haptic".into()];
        let capabilities = vec![(
            make_node("mobile", Some("Mobile")),
            vec!["haptic".to_string()],
        )];
        let rows = vec![ReificationRow {
            aio_kind: "modal".into(),
            platform: make_node("web", Some("Web")),
            form_factor: make_node("screen", Some("Screen")),
            interaction_class: make_node("pointer", Some("Pointer")),
            cio: "HtmlDialog".into(),
        }];

        let report = judge_leaf_completeness(&intent_aios, &rows, &known_kinds, &capabilities);

        assert_eq!(report.reified.len(), 1);
        assert_eq!(report.reified[0].cio, "HtmlDialog");

        assert_eq!(report.unreifiable.len(), 2);
        assert_eq!(report.unreifiable[0].reason, UnreifiableReason::UnknownAioKind);
        assert_eq!(
            report.unreifiable[1].reason,
            UnreifiableReason::CapabilityAbsent
        );
    }

    #[test]
    fn vector_empty_context() {
        let report = judge_leaf_completeness(&[], &[], &[], &[]);
        assert!(report.reified.is_empty());
        assert!(report.unreifiable.is_empty());
    }
}
