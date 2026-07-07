//! Acceptance: bundles-load — every loom-spec reference bundle (single-select,
//! command, token-graph, composition containers, motion) loads through the
//! SHACL gate into its named graph with zero violations.

mod common;

use loom_graph::{
    catalog_inventory, load_bundle, BundleError, BundleLoadOutcome, InventoryState, LoomStore,
    NamedGraph,
};

#[test]
fn bundles_load_through_the_gate_with_zero_violations() {
    let store = LoomStore::new().unwrap();
    let loaded = common::load_reference_bundles(&store);

    assert_eq!(loaded.len(), common::REFERENCE_BUNDLES.len());
    for (bundle, (file, class)) in loaded.iter().zip(common::REFERENCE_BUNDLES) {
        assert_eq!(bundle.artifact_class, class, "{file}");
        assert_eq!(bundle.named_graph, format!("loom:g/{class}"), "{file}");
        assert!(bundle.triple_count > 0, "{file} loaded no triples");
        assert!(bundle.provenance.validated, "{file} provenance not validated");
    }

    // Every artifact-class graph is now populated.
    for class in loom_ontology::ARTIFACT_CLASSES {
        let graph = NamedGraph::from_artifact_class(class).unwrap();
        assert!(
            store.triple_count(&graph).unwrap() > 0,
            "loom:g/{class} is empty after loading the reference bundles"
        );
    }

    // The catalog inventory projects every class as present.
    let inventory = catalog_inventory(&store).unwrap();
    for class in loom_ontology::ARTIFACT_CLASSES {
        assert_eq!(
            inventory.classes[class],
            InventoryState::Present,
            "rm-catalog-inventory: {class} not present"
        );
    }
}

#[test]
fn bundles_load_rejects_reload_of_resident_bundle() {
    let store = LoomStore::new().unwrap();
    common::load_reference_bundles(&store);

    let err = load_bundle(&store, &common::request("token-graph.jsonld", "tokens"))
        .expect_err("re-load of a resident bundle must be rejected");
    assert!(matches!(
        err,
        BundleError::Invariant("inv-bundle-already-resident")
    ));
}

#[test]
fn bundles_load_rejects_unknown_artifact_class() {
    let store = LoomStore::new().unwrap();
    let err = load_bundle(&store, &common::request("token-graph.jsonld", "styles"))
        .expect_err("unknown artifact class must be rejected");
    assert!(matches!(
        err,
        BundleError::Invariant("inv-known-artifact-class")
    ));
}

#[test]
fn bundles_load_rejects_missing_source_path() {
    let store = LoomStore::new().unwrap();
    let mut request = common::request("token-graph.jsonld", "tokens");
    request.bundle_path = String::new();
    let err = load_bundle(&store, &request).expect_err("empty bundle path must be rejected");
    assert!(matches!(
        err,
        BundleError::Invariant("inv-bundle-source-present")
    ));
}

#[test]
fn bundles_load_fail_closed_on_violation() {
    let store = LoomStore::new().unwrap();
    let mut request = common::request("broken-motion.jsonld", "motion");
    request.bundle_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/broken-motion.jsonld"
    )
    .to_string();

    let outcome = load_bundle(&store, &request).unwrap();
    let BundleLoadOutcome::Rejected(rejected) = outcome else {
        panic!("a motion entry without a reduced-motion fallback must be rejected");
    };
    assert!(
        rejected
            .shacl_violations
            .iter()
            .any(|v| v.contains("reducedMotionFallback")),
        "violation report names the failing path: {:?}",
        rejected.shacl_violations
    );

    // Fail-closed: nothing entered loom:g/motion.
    assert_eq!(store.triple_count(&NamedGraph::Motion).unwrap(), 0);

    // Once fixed (the reference bundle), the same path family loads fine and
    // the rejected attempt does not block it (previously-rejected → reload ok).
    let outcome = load_bundle(&store, &common::request("motion.jsonld", "motion")).unwrap();
    assert!(matches!(outcome, BundleLoadOutcome::Loaded(_)));
}
