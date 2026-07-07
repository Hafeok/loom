//! Shared helpers: load every loom-spec reference bundle through the gate.

use std::path::PathBuf;

use loom_graph::{load_bundle, BundleLoadOutcome, BundleLoadRequest, BundleLoaded, LoomStore};

/// The loom-spec reference bundles, in load order: (file, artifact class).
pub const REFERENCE_BUNDLES: [(&str, &str); 7] = [
    ("token-graph.jsonld", "tokens"),
    ("single-select.machine.jsonld", "machines"),
    ("command.machine.jsonld", "machines"),
    ("composition-containers.jsonld", "machines"),
    ("motion.jsonld", "motion"),
    ("reification-table.jsonld", "reification"),
    ("mode-contracts.jsonld", "mode-contracts"),
];

pub fn bundles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/bundles")
        .canonicalize()
        .expect("reference/bundles exists")
}

pub fn request(file: &str, artifact_class: &str) -> BundleLoadRequest {
    BundleLoadRequest {
        bundle_path: bundles_dir().join(file).to_string_lossy().into_owned(),
        artifact_class: artifact_class.to_string(),
        source: "loom-spec".to_string(),
        version: "0.1.0".to_string(),
    }
}

/// Load every reference bundle; panic with the shape report on any rejection.
pub fn load_reference_bundles(store: &LoomStore) -> Vec<BundleLoaded> {
    REFERENCE_BUNDLES
        .iter()
        .map(|(file, class)| {
            match load_bundle(store, &request(file, class)).unwrap_or_else(|e| {
                panic!("{file} ({class}) failed to load: {e}");
            }) {
                BundleLoadOutcome::Loaded(loaded) => loaded,
                BundleLoadOutcome::Rejected(rejected) => panic!(
                    "{file} ({class}) rejected by the SHACL gate:\n  {}",
                    rejected.shacl_violations.join("\n  ")
                ),
            }
        })
        .collect()
}
