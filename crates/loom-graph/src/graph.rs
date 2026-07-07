//! Named graphs — one per artifact class, plus catalog records and run graphs.

use std::fmt;

/// A named graph in the loom store. The five artifact-class graphs mirror
/// e-bundle-decider's `inv-known-artifact-class`; `Catalog` holds bundle load
/// records and `Run` holds PROV-O run graphs (`loom:g/runs/<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamedGraph {
    Tokens,
    Machines,
    Reification,
    Motion,
    ModeContracts,
    Catalog,
    Run(String),
}

impl NamedGraph {
    /// The graph IRI in the store.
    pub fn iri(&self) -> String {
        match self {
            Self::Run(id) => format!("{}runs/{id}", loom_ontology::GRAPH_NS),
            other => format!("{}{}", loom_ontology::GRAPH_NS, other.class_segment()),
        }
    }

    /// The display name used across the spec and event payloads, e.g. `loom:g/tokens`.
    pub fn qname(&self) -> String {
        match self {
            Self::Run(id) => format!("loom:g/runs/{id}"),
            other => format!("loom:g/{}", other.class_segment()),
        }
    }

    /// The artifact class targeted by `cmd-load-bundle`, if this graph holds one.
    pub fn artifact_class(&self) -> Option<&str> {
        match self {
            Self::Catalog | Self::Run(_) => None,
            other => Some(other.class_segment()),
        }
    }

    /// Resolve an artifact class declared on a bundle to its target graph.
    /// Unknown classes return `None` — the caller must reject fail-closed.
    pub fn from_artifact_class(class: &str) -> Option<Self> {
        match class {
            "tokens" => Some(Self::Tokens),
            "machines" => Some(Self::Machines),
            "reification" => Some(Self::Reification),
            "motion" => Some(Self::Motion),
            "mode-contracts" => Some(Self::ModeContracts),
            _ => None,
        }
    }

    fn class_segment(&self) -> &str {
        match self {
            Self::Tokens => "tokens",
            Self::Machines => "machines",
            Self::Reification => "reification",
            Self::Motion => "motion",
            Self::ModeContracts => "mode-contracts",
            Self::Catalog => "catalog",
            Self::Run(_) => unreachable!("Run graphs are formatted by iri()/qname()"),
        }
    }
}

impl fmt::Display for NamedGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.qname())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_classes_round_trip() {
        for class in loom_ontology::ARTIFACT_CLASSES {
            let g = NamedGraph::from_artifact_class(class).expect("known class resolves");
            assert_eq!(g.artifact_class(), Some(class));
            assert_eq!(g.qname(), format!("loom:g/{class}"));
        }
        assert_eq!(NamedGraph::from_artifact_class("styles"), None);
    }

    #[test]
    fn run_graph_iri() {
        let g = NamedGraph::Run("r1".into());
        assert_eq!(g.iri(), "http://loom.dev/g/runs/r1");
        assert_eq!(g.qname(), "loom:g/runs/r1");
    }
}
