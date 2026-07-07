//! Acceptance: shacl-vectors-green — shapes/ assertions pass on all
//! encoding-neutral vectors: negative fixtures rejected, positive fixtures
//! admitted, zero MISMATCH (p-vectors-equivalence).

use loom_graph::bundle::load_gate::parse_against_pinned_context;
use loom_graph::decider::{self, BundleDeciderState, BundleEvent, Decision, LoadBundleCommand};
use loom_graph::motion::{judge_motion_references, judge_reduced_motion_fallback, MotionReference};
use loom_graph::reification_table::{judge_row_ambiguity, NodeRef, ReificationRow};
use loom_graph::shacl::ShapeSet;
use loom_graph::tokens::{judge_token_resolution, TokenRow};
use loom_graph::vectors::{load_all, VectorTest};
use loom_graph::NamedGraph;
use serde_json::{json, Value};

#[test]
fn shacl_vectors_zero_mismatch() {
    let files = load_all().expect("vectors/ loads");
    assert!(!files.is_empty(), "no vector files found");

    let mut mismatches = Vec::new();
    let mut total = 0;
    for (stem, file) in files {
        for vector in file.vectors {
            total += 1;
            let actual = run_vector(&vector);
            let Value::Object(expected) = &vector.expected else {
                panic!("{stem}/{}: expected must be an object", vector.id);
            };
            for (key, expected_value) in expected {
                let actual_value = actual.get(key).cloned().unwrap_or(Value::Null);
                if actual_value != *expected_value {
                    mismatches.push(format!(
                        "MISMATCH {stem}/{} on '{key}': expected {expected_value}, got {actual_value}",
                        vector.id
                    ));
                }
            }
        }
    }

    assert!(total >= 15, "vector suite unexpectedly small: {total}");
    assert!(
        mismatches.is_empty(),
        "{} vector verdict mismatch(es):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Compute every verdict a vector can ask about, keyed like the expected object.
fn run_vector(vector: &VectorTest) -> serde_json::Map<String, Value> {
    let fixture = &vector.fixture;
    match fixture.get("type").and_then(Value::as_str) {
        Some("MotionEntry") => motion_entry_verdicts(fixture),
        Some("TokenGraph") => token_graph_verdicts(fixture),
        Some("ReificationTable") => reification_verdicts(fixture),
        Some("BundleDecider") => decider_verdicts(fixture),
        _ if fixture.get("motion_entries").is_some() => motion_reference_verdicts(fixture),
        _ if fixture.get("artifact_class").is_some() => generic_shacl_verdicts(fixture),
        other => panic!("vector {} has unrecognized fixture kind {other:?}", vector.id),
    }
}

fn shacl_passes(artifact_class: &str, graph_nodes: Value) -> bool {
    let doc = json!({ "@graph": graph_nodes }).to_string();
    let graph = NamedGraph::from_artifact_class(artifact_class).expect("known class");
    let quads = parse_against_pinned_context(&doc, &graph).expect("fixture JSON-LD parses");
    let shapes = ShapeSet::for_artifact_class(artifact_class).expect("shapes load");
    shapes.validate(&quads).expect("validation runs").is_empty()
}

fn motion_entry_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let name = fixture["name"].as_str().unwrap_or("unnamed");
    let node = json!([{
        "id": format!("motion:{}", name.replace(' ', "-")),
        "type": "MotionEntry",
        "name": fixture["name"],
        "communicates": fixture["communicates"],
        "duration": fixture["duration"],
        "easing": fixture["easing"],
        "reducedMotionFallback": fixture["reduced_motion_fallback"],
    }]);
    let fallback = fixture["reduced_motion_fallback"].as_str().unwrap_or("");
    let mut out = serde_json::Map::new();
    out.insert("shacl_passes".into(), shacl_passes("motion", node).into());
    out.insert(
        "wcag_2_3_3".into(),
        judge_reduced_motion_fallback(fallback).into(),
    );
    out
}

fn motion_reference_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let entry_names: Vec<String> = fixture["motion_entries"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|e| e["name"].as_str().map(String::from))
        .collect();
    let references: Vec<MotionReference> = fixture["transitions"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|t| MotionReference {
            component: t["component"].as_str().unwrap_or("").to_string(),
            carries_motion: t["carries_motion"].as_str().map(String::from),
        })
        .collect();
    let report = judge_motion_references(&entry_names, &references);
    let mut out = serde_json::Map::new();
    out.insert("motion_check_passes".into(), report.passes.into());
    out.insert(
        "dangling_references".into(),
        serde_json::to_value(&report.dangling).unwrap(),
    );
    out
}

fn token_graph_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let tokens = fixture["tokens"].as_array().cloned().unwrap_or_default();

    // Encoding-neutral fixture → JSON-LD for the shape assertion.
    let nodes: Vec<Value> = tokens
        .iter()
        .map(|t| {
            let tier = t["tier"].as_str().unwrap_or("");
            let class = match tier {
                "primitive" => "PrimitiveToken",
                "semantic" => "SemanticToken",
                _ => "ComponentToken",
            };
            let mut node = serde_json::Map::new();
            node.insert("id".into(), json!(format!("token:{}", t["id"].as_str().unwrap_or("anon"))));
            node.insert("type".into(), json!(class));
            if let Some(v) = t.get("value") {
                node.insert("value".into(), v.clone());
            }
            if let Some(b) = t["binds"].as_str() {
                node.insert("binds".into(), json!(format!("token:{b}")));
            }
            if let Some(c) = t.get("component").filter(|c| !c.is_null()) {
                node.insert("component".into(), c.clone());
            }
            Value::Object(node)
        })
        .collect();

    // The same fixture → rows for the resolution oracle.
    let rows: Vec<TokenRow> = tokens
        .iter()
        .map(|t| TokenRow {
            token: t["id"].as_str().unwrap_or("").to_string(),
            tier: t["tier"].as_str().unwrap_or("").to_string(),
            binds: t["binds"].as_str().map(String::from),
            component: t["component"].as_str().map(String::from),
        })
        .collect();

    let mut out = serde_json::Map::new();
    out.insert("shacl_passes".into(), shacl_passes("tokens", json!(nodes)).into());
    out.insert(
        "resolves".into(),
        judge_token_resolution(&rows).is_empty().into(),
    );
    out
}

fn reification_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let raw_rows = fixture["rows"].as_array().cloned().unwrap_or_default();

    let nodes: Vec<Value> = raw_rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut node = serde_json::Map::new();
            node.insert("id".into(), json!(format!("row:vector-{i}")));
            node.insert("type".into(), json!("ReificationRow"));
            if let Some(k) = r["aio_kind"].as_str() {
                node.insert("aioKind".into(), json!(k));
            }
            for (fixture_key, term) in [
                ("platform", "platform"),
                ("form_factor", "formFactor"),
                ("interaction_class", "interactionClass"),
            ] {
                if let Some(v) = r[fixture_key].as_str() {
                    node.insert(term.into(), json!({ "id": format!("ctx:{v}") }));
                }
            }
            if let Some(c) = r["cio"].as_str() {
                node.insert("cio".into(), json!(c));
            }
            Value::Object(node)
        })
        .collect();

    let node = |value: &Value| NodeRef {
        node: value.as_str().unwrap_or("").to_string(),
        label: None,
    };
    let rows: Vec<ReificationRow> = raw_rows
        .iter()
        .map(|r| ReificationRow {
            aio_kind: r["aio_kind"].as_str().unwrap_or("").to_string(),
            platform: node(&r["platform"]),
            form_factor: node(&r["form_factor"]),
            interaction_class: node(&r["interaction_class"]),
            cio: r["cio"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    let mut out = serde_json::Map::new();
    out.insert(
        "shacl_passes".into(),
        shacl_passes("reification", json!(nodes)).into(),
    );
    out.insert(
        "unambiguous".into(),
        judge_row_ambiguity(&rows).is_empty().into(),
    );
    out
}

fn decider_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let given: Vec<BundleEvent> =
        serde_json::from_value(fixture["given"].clone()).expect("given events parse");
    let command: LoadBundleCommand =
        serde_json::from_value(fixture["command"].clone()).expect("command parses");
    let state = BundleDeciderState::from_history(given.iter());

    let mut out = serde_json::Map::new();
    match decider::decide(&state, &command) {
        Decision::Emit(events) => {
            out.insert("emit".into(), serde_json::to_value(&events).unwrap());
        }
        Decision::Reject(invariant) => {
            out.insert("reject".into(), json!(invariant));
        }
    }
    out
}

fn generic_shacl_verdicts(fixture: &Value) -> serde_json::Map<String, Value> {
    let class = fixture["artifact_class"].as_str().expect("artifact_class");
    let mut out = serde_json::Map::new();
    out.insert(
        "shacl_passes".into(),
        shacl_passes(class, fixture["graph"].clone()).into(),
    );
    out
}
