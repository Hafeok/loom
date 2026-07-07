//! e-bundle aggregate slice
//! Implements the load gate per pat-load-gate, typed views per pat-typed-row-view,
//! and PROV-O provenance recording per pat-run-recorder.

pub mod load_gate;
pub mod provenance;
pub mod views;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleLoadRequest {
    pub bundle_path: String,
    pub artifact_class: String,
    pub source: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleLoadOutcome {
    Loaded(BundleLoaded),
    Rejected(BundleRejected),
}

/// ev-bundle-loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleLoaded {
    pub bundle_id: String,
    pub artifact_class: String,
    pub named_graph: String,
    pub triple_count: usize,
    pub provenance: ProvenanceRecord,
}

/// ev-bundle-rejected. Nothing entered the store (p-fail-closed-load).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRejected {
    pub bundle_id: String,
    pub artifact_class: String,
    pub shacl_violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub activity_id: String,
    pub entity_id: String,
    pub generated_at: String,
    pub validated: bool,
}

#[derive(Error, Debug)]
pub enum BundleError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON-LD parse error: {0}")]
    JsonLd(String),
    #[error("store error: {0}")]
    Store(#[from] oxigraph::store::StorageError),
    #[error("shapes error: {0}")]
    Shapes(#[from] crate::shacl::ShaclError),
    #[error("invariant rejected: {0}")]
    Invariant(&'static str),
    #[error("query error: {0}")]
    Query(#[from] crate::query::QueryError),
}
