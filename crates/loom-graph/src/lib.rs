//! loom-graph — the graph core (deliverable d1-graph-core, feature ft-graph-core).
//!
//! The embedded Oxigraph store is the primary store: tokens, machines, the
//! reification table, the motion catalog and the mode-contracts are RDF,
//! loaded once through the fail-closed SHACL gate and queried everywhere
//! (d-graph-primary). Rust structs are views over the graph; queries fetch,
//! oracles judge; every run verdict is appended to the run graph.

pub mod bundle;
pub mod decider;
pub mod graph;
pub mod mode_contracts;
pub mod motion;
pub mod query;
pub mod reification_table;
pub mod shacl;
pub mod sparql;
pub mod store;
pub mod tokens;
pub mod vectors;

pub use bundle::load_gate::load_bundle;
pub use bundle::views::{catalog_inventory, CatalogInventory, InventoryState};
pub use bundle::{BundleError, BundleLoadOutcome, BundleLoadRequest, BundleLoaded, BundleRejected};
pub use graph::NamedGraph;
pub use query::{QueryError, QueryExecutor};
pub use store::LoomStore;
