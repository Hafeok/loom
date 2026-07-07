//! e-bundle-decider — the pure decision procedure for `cmd-load-bundle`.
//!
//! The load gate composes this with parsing and SHACL; the decider itself is
//! side-effect free so the behaviour scenarios run as plain unit tests and as
//! encoding-neutral vectors.

use serde::{Deserialize, Serialize};

use crate::graph::NamedGraph;

pub const INV_KNOWN_ARTIFACT_CLASS: &str = "inv-known-artifact-class";
pub const INV_BUNDLE_SOURCE_PRESENT: &str = "inv-bundle-source-present";
pub const INV_BUNDLE_ALREADY_RESIDENT: &str = "inv-bundle-already-resident";

/// cmd-load-bundle. Wire form is the spec's kebab-case field names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoadBundleCommand {
    pub bundle_path: String,
    pub artifact_class: String,
}

/// Events the decider evolves from and emits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum BundleEvent {
    #[serde(rename = "ev-bundle-loaded", rename_all = "kebab-case")]
    Loaded {
        bundle_id: String,
        artifact_class: String,
        named_graph: String,
    },
    #[serde(rename = "ev-bundle-rejected", rename_all = "kebab-case")]
    Rejected {
        bundle_id: String,
        artifact_class: String,
        shacl_violations: String,
    },
}

/// Decider state, evolved from the bundle's event history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleDeciderState {
    pub shacl_status: ShaclStatus,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaclStatus {
    None,
    Conformant,
    Violated,
}

impl Default for BundleDeciderState {
    fn default() -> Self {
        Self {
            shacl_status: ShaclStatus::None,
            bundle_id: String::new(),
        }
    }
}

impl BundleDeciderState {
    pub fn evolve(mut self, event: &BundleEvent) -> Self {
        match event {
            BundleEvent::Loaded { bundle_id, .. } => {
                self.shacl_status = ShaclStatus::Conformant;
                self.bundle_id = bundle_id.clone();
            }
            BundleEvent::Rejected { bundle_id, .. } => {
                self.shacl_status = ShaclStatus::Violated;
                self.bundle_id = bundle_id.clone();
            }
        }
        self
    }

    pub fn from_history<'a>(events: impl IntoIterator<Item = &'a BundleEvent>) -> Self {
        events.into_iter().fold(Self::default(), Self::evolve)
    }
}

/// The decider's verdict: emit events, or reject on an invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Emit(Vec<BundleEvent>),
    Reject(&'static str),
}

/// Pure decide — guards mirror e-bundle-decider.yaml in order.
pub fn decide(state: &BundleDeciderState, command: &LoadBundleCommand) -> Decision {
    if command.bundle_path.is_empty() {
        return Decision::Reject(INV_BUNDLE_SOURCE_PRESENT);
    }
    let Some(graph) = NamedGraph::from_artifact_class(&command.artifact_class) else {
        return Decision::Reject(INV_KNOWN_ARTIFACT_CLASS);
    };
    if state.shacl_status == ShaclStatus::Conformant && state.bundle_id == command.bundle_path {
        return Decision::Reject(INV_BUNDLE_ALREADY_RESIDENT);
    }
    Decision::Emit(vec![BundleEvent::Loaded {
        bundle_id: command.bundle_path.clone(),
        artifact_class: command.artifact_class.clone(),
        named_graph: graph.qname(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(path: &str, class: &str) -> LoadBundleCommand {
        LoadBundleCommand {
            bundle_path: path.into(),
            artifact_class: class.into(),
        }
    }

    #[test]
    fn conformant_token_bundle_loads_into_its_named_graph() {
        let decision = decide(&BundleDeciderState::default(), &cmd("bundles/tokens.jsonld", "tokens"));
        assert_eq!(
            decision,
            Decision::Emit(vec![BundleEvent::Loaded {
                bundle_id: "bundles/tokens.jsonld".into(),
                artifact_class: "tokens".into(),
                named_graph: "loom:g/tokens".into(),
            }])
        );
    }

    #[test]
    fn unknown_artifact_class_is_rejected_fail_closed() {
        let decision = decide(&BundleDeciderState::default(), &cmd("bundles/styles.jsonld", "styles"));
        assert_eq!(decision, Decision::Reject(INV_KNOWN_ARTIFACT_CLASS));
    }

    #[test]
    fn bundle_without_source_path_is_rejected() {
        let decision = decide(&BundleDeciderState::default(), &cmd("", "tokens"));
        assert_eq!(decision, Decision::Reject(INV_BUNDLE_SOURCE_PRESENT));
    }

    #[test]
    fn previously_rejected_bundle_may_be_reloaded_once_fixed() {
        let state = BundleDeciderState::from_history([&BundleEvent::Rejected {
            bundle_id: "bundles/machines.jsonld".into(),
            artifact_class: "machines".into(),
            shacl_violations: "sh:MinCountConstraintComponent on loom:onEvent".into(),
        }]);
        let decision = decide(&state, &cmd("bundles/machines.jsonld", "machines"));
        assert_eq!(
            decision,
            Decision::Emit(vec![BundleEvent::Loaded {
                bundle_id: "bundles/machines.jsonld".into(),
                artifact_class: "machines".into(),
                named_graph: "loom:g/machines".into(),
            }])
        );
    }

    #[test]
    fn reloading_the_already_resident_bundle_is_a_rejected_no_op() {
        let state = BundleDeciderState::from_history([&BundleEvent::Loaded {
            bundle_id: "bundles/tokens.jsonld".into(),
            artifact_class: "tokens".into(),
            named_graph: "loom:g/tokens".into(),
        }]);
        let decision = decide(&state, &cmd("bundles/tokens.jsonld", "tokens"));
        assert_eq!(decision, Decision::Reject(INV_BUNDLE_ALREADY_RESIDENT));
    }
}
