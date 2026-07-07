//! pat-run-recorder: the run is a `prov:Activity`, the bundle an
//! `prov:Entity`; the verdict is appended to `loom:g/runs/<id>` as produced,
//! and the catalog record (load status + provenance) rides in the same
//! atomic write.

use std::time::{SystemTime, UNIX_EPOCH};

use oxigraph::model::{Literal, NamedNode, Quad};

use crate::bundle::{BundleError, BundleLoadRequest, ProvenanceRecord};
use crate::graph::NamedGraph;

fn loom(term: &str) -> NamedNode {
    NamedNode::new(format!("{}{term}", loom_ontology::LOOM_NS)).expect("valid loom IRI")
}

fn prov(term: &str) -> NamedNode {
    NamedNode::new(format!("{}{term}", loom_ontology::PROV_NS)).expect("valid prov IRI")
}

fn rdf_type() -> NamedNode {
    NamedNode::new(format!("{}type", loom_ontology::RDF_NS)).expect("valid rdf IRI")
}

fn sanitize(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Build the PROV-O run quads and the catalog record for one load attempt.
/// Returns the record plus the quads; the caller owns the atomic write so a
/// conformant bundle and its record land in one transaction.
pub fn record_load(
    request: &BundleLoadRequest,
    graph: &NamedGraph,
    validated: bool,
    violations: &[String],
    triple_count: usize,
) -> Result<(ProvenanceRecord, Vec<Quad>), BundleError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch");
    let run_id = format!("{}-{}", now.as_nanos(), sanitize(&request.bundle_path));
    let run_graph = NamedGraph::Run(run_id.clone());
    let run_graph_node = NamedNode::new(run_graph.iri()).expect("valid run graph IRI");

    let activity = NamedNode::new(format!("{}activity/{run_id}", loom_ontology::RUN_NS))
        .expect("valid activity IRI");
    let entity = NamedNode::new(format!("{}entity/{run_id}", loom_ontology::RUN_NS))
        .expect("valid entity IRI");
    let verdict = if validated { "conformant" } else { "violated" };
    let generated_at = now.as_secs().to_string();

    let mut quads = vec![
        Quad::new(activity.clone(), rdf_type(), prov("Activity"), run_graph_node.clone()),
        Quad::new(activity.clone(), loom("check"), Literal::new_simple_literal("shacl-load-gate"), run_graph_node.clone()),
        Quad::new(activity.clone(), prov("generated"), entity.clone(), run_graph_node.clone()),
        Quad::new(activity.clone(), prov("endedAtTime"), Literal::new_simple_literal(&generated_at), run_graph_node.clone()),
        Quad::new(activity.clone(), loom("verdict"), Literal::new_simple_literal(verdict), run_graph_node.clone()),
        Quad::new(entity.clone(), rdf_type(), prov("Entity"), run_graph_node.clone()),
        Quad::new(entity.clone(), loom("bundleId"), Literal::new_simple_literal(&request.bundle_path), run_graph_node.clone()),
        Quad::new(entity.clone(), loom("artifactClass"), Literal::new_simple_literal(&request.artifact_class), run_graph_node.clone()),
        Quad::new(entity.clone(), loom("source"), Literal::new_simple_literal(&request.source), run_graph_node.clone()),
        Quad::new(entity.clone(), loom("version"), Literal::new_simple_literal(&request.version), run_graph_node.clone()),
    ];
    for (i, violation) in violations.iter().enumerate() {
        let v = NamedNode::new(format!("{}violation/{run_id}/{i}", loom_ontology::RUN_NS))
            .expect("valid violation IRI");
        quads.push(Quad::new(activity.clone(), loom("violation"), v.clone(), run_graph_node.clone()));
        quads.push(Quad::new(v, loom("wire"), Literal::new_simple_literal(violation), run_graph_node.clone()));
    }

    // Catalog record — the bundle's load status and provenance are part of
    // the catalog's record (e-bundle).
    let catalog_node =
        NamedNode::new(NamedGraph::Catalog.iri()).expect("valid catalog graph IRI");
    let record = NamedNode::new(format!("{}record/{run_id}", loom_ontology::RUN_NS))
        .expect("valid record IRI");
    let xsd_integer = NamedNode::new(format!("{}integer", loom_ontology::XSD_NS))
        .expect("valid xsd IRI");
    quads.extend([
        Quad::new(record.clone(), rdf_type(), loom("BundleRecord"), catalog_node.clone()),
        Quad::new(record.clone(), loom("bundleId"), Literal::new_simple_literal(&request.bundle_path), catalog_node.clone()),
        Quad::new(record.clone(), loom("artifactClass"), Literal::new_simple_literal(&request.artifact_class), catalog_node.clone()),
        Quad::new(record.clone(), loom("namedGraph"), Literal::new_simple_literal(graph.qname()), catalog_node.clone()),
        Quad::new(record.clone(), loom("shaclStatus"), Literal::new_simple_literal(verdict), catalog_node.clone()),
        Quad::new(record.clone(), loom("tripleCount"), Literal::new_typed_literal(triple_count.to_string(), xsd_integer), catalog_node.clone()),
        Quad::new(record, loom("run"), activity.clone(), catalog_node),
    ]);

    Ok((
        ProvenanceRecord {
            activity_id: activity.as_str().to_string(),
            entity_id: entity.as_str().to_string(),
            generated_at,
            validated,
        },
        quads,
    ))
}
