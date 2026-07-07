//! rm-catalog-inventory — a typed view over the catalog graph
//! (pat-typed-row-view): query file + serde row + fold to per-class state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::BundleError;
use crate::graph::NamedGraph;
use crate::query::QueryExecutor;
use crate::store::LoomStore;

const CATALOG_INVENTORY_QUERY: &str = include_str!("../../queries/catalog_inventory.sparql");

/// One catalog record row, as fetched.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogRecordRow {
    pub bundle_id: String,
    pub artifact_class: String,
    pub shacl_status: String,
    #[serde(default)]
    pub triple_count: Option<i64>,
}

/// rm-catalog-inventory states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InventoryState {
    Loading,
    Present,
    Empty,
    Failed,
}

/// The inventory projection: one state per artifact class.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogInventory {
    pub classes: BTreeMap<String, InventoryState>,
    pub records: Vec<InventoryRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryRecord {
    pub bundle_id: String,
    pub artifact_class: String,
    pub shacl_status: String,
    pub triple_count: i64,
}

impl From<CatalogRecordRow> for InventoryRecord {
    fn from(row: CatalogRecordRow) -> Self {
        Self {
            bundle_id: row.bundle_id,
            artifact_class: row.artifact_class,
            shacl_status: row.shacl_status,
            triple_count: row.triple_count.unwrap_or(0),
        }
    }
}

/// Project the catalog inventory from the store.
pub fn catalog_inventory(store: &LoomStore) -> Result<CatalogInventory, BundleError> {
    let rows: Vec<CatalogRecordRow> =
        store.fetch_rows(&NamedGraph::Catalog, CATALOG_INVENTORY_QUERY)?;

    let mut classes: BTreeMap<String, InventoryState> = loom_ontology::ARTIFACT_CLASSES
        .iter()
        .map(|c| (c.to_string(), InventoryState::Empty))
        .collect();
    for row in &rows {
        let state = classes
            .entry(row.artifact_class.clone())
            .or_insert(InventoryState::Empty);
        if row.shacl_status == "conformant" {
            *state = InventoryState::Present;
        } else if *state == InventoryState::Empty {
            *state = InventoryState::Failed;
        }
    }

    Ok(CatalogInventory {
        classes,
        records: rows.into_iter().map(InventoryRecord::from).collect(),
    })
}
