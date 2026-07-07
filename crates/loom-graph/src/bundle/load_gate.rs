//! pat-load-gate: parse JSON-LD against the pinned context → SHACL-validate
//! against shapes/ → atomic insert into the target named graph. On violation,
//! return the shape report and write nothing of the bundle (p-fail-closed-load).

use std::path::Path;

use oxigraph::io::{JsonLdProfileSet, RdfFormat, RdfParser};
use oxigraph::model::{NamedNode, Quad};
use serde::Deserialize;

use crate::bundle::{provenance, BundleError, BundleLoadOutcome, BundleLoaded, BundleRejected, BundleLoadRequest};
use crate::decider::{self, BundleDeciderState, Decision, LoadBundleCommand, ShaclStatus};
use crate::graph::NamedGraph;
use crate::shacl::ShapeSet;
use crate::store::LoomStore;

/// Maintained query: the catalog record status for one bundle id.
const BUNDLE_STATUS_QUERY: &str = include_str!("../../queries/bundle_status.sparql");

#[derive(Debug, Deserialize)]
struct StatusRow {
    shacl_status: String,
}

/// Load one bundle through the gate. The decider guards run first; a guard
/// rejection is an `Err(Invariant)` — no event, nothing written. A SHACL
/// violation is an emitted `ev-bundle-rejected`: the verdict is recorded in
/// the catalog and run graph, but no bundle triple enters the store.
pub fn load_bundle(
    store: &LoomStore,
    request: &BundleLoadRequest,
) -> Result<BundleLoadOutcome, BundleError> {
    // 1. Evolve decider state from the catalog's record for this bundle id.
    let state = decider_state(store, &request.bundle_path)?;
    let command = LoadBundleCommand {
        bundle_path: request.bundle_path.clone(),
        artifact_class: request.artifact_class.clone(),
    };
    let graph = match decider::decide(&state, &command) {
        Decision::Reject(invariant) => return Err(BundleError::Invariant(invariant)),
        Decision::Emit(_) => NamedGraph::from_artifact_class(&request.artifact_class)
            .expect("decider admitted the artifact class"),
    };

    // 2. Parse JSON-LD against the pinned context.
    let raw = std::fs::read_to_string(Path::new(&request.bundle_path))?;
    let quads = parse_against_pinned_context(&raw, &graph)?;

    // 3. SHACL-validate against shapes/.
    let shapes = ShapeSet::for_artifact_class(&request.artifact_class)?;
    let violations = shapes.validate(&quads)?;

    if !violations.is_empty() {
        // Fail closed: record the verdict, write no bundle triple.
        let wire: Vec<String> = violations.iter().map(|v| v.wire()).collect();
        let (_, record_quads) = provenance::record_load(request, &graph, false, &wire, 0)?;
        store.insert_quads(record_quads)?;
        return Ok(BundleLoadOutcome::Rejected(BundleRejected {
            bundle_id: request.bundle_path.clone(),
            artifact_class: request.artifact_class.clone(),
            shacl_violations: wire,
        }));
    }

    // 4. Atomic insert: bundle triples + catalog record + PROV-O run, one transaction.
    let triple_count = quads.len();
    let (record, record_quads) = provenance::record_load(request, &graph, true, &[], triple_count)?;
    let mut all = quads;
    all.extend(record_quads);
    store.insert_quads(all)?;

    Ok(BundleLoadOutcome::Loaded(BundleLoaded {
        bundle_id: request.bundle_path.clone(),
        artifact_class: request.artifact_class.clone(),
        named_graph: graph.qname(),
        triple_count,
        provenance: record,
    }))
}

fn decider_state(store: &LoomStore, bundle_id: &str) -> Result<BundleDeciderState, BundleError> {
    let query = crate::sparql::SparqlBuilder::from_query_file(BUNDLE_STATUS_QUERY)
        .bind_graph("graph", &NamedGraph::Catalog)
        .bind_str("bundle_id", bundle_id)
        .build();
    let rows: Vec<StatusRow> = store.fetch_rows_bound(&query)?;
    let status = if rows.iter().any(|r| r.shacl_status == "conformant") {
        ShaclStatus::Conformant
    } else if rows.iter().any(|r| r.shacl_status == "violated") {
        ShaclStatus::Violated
    } else {
        ShaclStatus::None
    };
    Ok(BundleDeciderState {
        shacl_status: status,
        bundle_id: if status == ShaclStatus::None {
            String::new()
        } else {
            bundle_id.to_string()
        },
    })
}

/// Expand a bundle document against the pinned context (any context the
/// document carries is replaced, never trusted) and parse to quads targeted
/// at the bundle's named graph.
pub fn parse_against_pinned_context(
    raw: &str,
    graph: &NamedGraph,
) -> Result<Vec<Quad>, BundleError> {
    let mut doc: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| BundleError::JsonLd(e.to_string()))?;
    let serde_json::Value::Object(obj) = &mut doc else {
        return Err(BundleError::JsonLd("bundle is not a JSON object".into()));
    };
    obj.insert("@context".into(), loom_ontology::pinned_context_value());
    let pinned = serde_json::to_string(&doc).map_err(|e| BundleError::JsonLd(e.to_string()))?;

    let parser = RdfParser::from_format(RdfFormat::JsonLd {
        profile: JsonLdProfileSet::empty(),
    });
    let quads = parser
        .for_slice(pinned.as_bytes())
        .collect::<Result<Vec<Quad>, _>>()
        .map_err(|e| BundleError::JsonLd(e.to_string()))?;

    let graph_node = NamedNode::new(graph.iri()).expect("named graph IRIs are valid");
    Ok(quads
        .into_iter()
        .map(|q| Quad {
            graph_name: graph_node.clone().into(),
            ..q
        })
        .collect())
}
