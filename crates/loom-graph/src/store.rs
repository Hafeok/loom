//! The embedded Oxigraph store — the primary store (d-graph-primary).
//!
//! Only this crate depends on oxigraph (s-store-single-owner); every other
//! crate reaches data through the typed-view API.

use oxigraph::model::{GraphNameRef, NamedNode, Quad};
use oxigraph::store::{StorageError, Store};

use crate::graph::NamedGraph;

/// One store per distribution; named graph per artifact class.
pub struct LoomStore {
    store: Store,
}

impl LoomStore {
    pub fn new() -> Result<Self, StorageError> {
        Ok(Self {
            store: Store::new()?,
        })
    }

    pub(crate) fn store(&self) -> &Store {
        &self.store
    }

    /// Atomically insert a batch of quads. This is the only write path:
    /// the load gate hands over quads only after SHACL passed (p-fail-closed-load).
    pub fn insert_quads(&self, quads: Vec<Quad>) -> Result<(), StorageError> {
        self.store.extend(quads)
    }

    /// Number of triples currently resident in `graph`.
    pub fn triple_count(&self, graph: &NamedGraph) -> Result<usize, StorageError> {
        let name = NamedNode::new(graph.iri()).map_err(|e| {
            StorageError::Other(format!("invalid graph IRI {}: {e}", graph.iri()).into())
        })?;
        let graph_ref = GraphNameRef::NamedNode(name.as_ref().into());
        let mut count = 0usize;
        for quad in self
            .store
            .quads_for_pattern(None, None, None, Some(graph_ref))
        {
            quad?;
            count += 1;
        }
        Ok(count)
    }
}
