//! §6.3 conformance runner for e-bundle-decider.
//!
//! `product decider conform e-bundle-decider --runner <this bin>` feeds the
//! Decider's scenarios as a JSON array of `{given, when}` requests on stdin;
//! for each we replay `given` through the realised state, decide `when`, and
//! answer `{emit: [...]}` or `{reject: "inv-..."}` — the oracle then compares
//! these outcomes payload-for-payload against the simulated Decider.

use loom_graph::decider::{decide, BundleDeciderState, BundleEvent, Decision, LoadBundleCommand};
use serde_json::{json, Value};

fn wire_str(with: Option<&Value>, key: &str) -> String {
    with.and_then(|w| w.get(key)).and_then(Value::as_str).unwrap_or("").to_string()
}

fn event_from_wire(v: &Value) -> Option<BundleEvent> {
    let (id, with) = match v {
        Value::String(s) => (s.as_str(), None),
        Value::Object(o) => (o.get("event")?.as_str()?, o.get("with")),
        _ => return None,
    };
    match id {
        "ev-bundle-loaded" => Some(BundleEvent::Loaded {
            bundle_id: wire_str(with, "bundle-id"),
            artifact_class: wire_str(with, "artifact-class"),
            named_graph: wire_str(with, "named-graph"),
        }),
        "ev-bundle-rejected" => Some(BundleEvent::Rejected {
            bundle_id: wire_str(with, "bundle-id"),
            artifact_class: wire_str(with, "artifact-class"),
            shacl_violations: wire_str(with, "shacl-violations"),
        }),
        _ => None,
    }
}

fn command_from_wire(v: &Value) -> Option<LoadBundleCommand> {
    let (id, with) = match v {
        Value::String(s) => (s.as_str(), None),
        Value::Object(o) => (o.get("command")?.as_str()?, o.get("with")),
        _ => return None,
    };
    (id == "cmd-load-bundle").then(|| LoadBundleCommand {
        bundle_path: wire_str(with, "bundle-path"),
        artifact_class: wire_str(with, "artifact-class"),
    })
}

fn event_to_wire(e: &BundleEvent) -> Value {
    match e {
        BundleEvent::Loaded { bundle_id, artifact_class, named_graph } => json!({
            "event": "ev-bundle-loaded",
            "with": { "bundle-id": bundle_id, "artifact-class": artifact_class, "named-graph": named_graph },
        }),
        BundleEvent::Rejected { bundle_id, artifact_class, shacl_violations } => json!({
            "event": "ev-bundle-rejected",
            "with": { "bundle-id": bundle_id, "artifact-class": artifact_class, "shacl-violations": shacl_violations },
        }),
    }
}

fn respond(request: &Value) -> Value {
    let given: Vec<BundleEvent> = request
        .get("given")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(event_from_wire).collect())
        .unwrap_or_default();
    let Some(command) = request.get("when").and_then(command_from_wire) else {
        return json!({ "reject": "unknown-command" });
    };
    let state = BundleDeciderState::from_history(given.iter());
    match decide(&state, &command) {
        Decision::Emit(events) => json!({ "emit": events.iter().map(event_to_wire).collect::<Vec<_>>() }),
        Decision::Reject(inv) => json!({ "reject": inv }),
    }
}

fn main() {
    let input = std::io::read_to_string(std::io::stdin()).expect("read stdin");
    let requests: Vec<Value> = serde_json::from_str(&input).expect("stdin is a JSON array of {given, when}");
    let responses: Vec<Value> = requests.iter().map(respond).collect();
    println!("{}", serde_json::to_string(&responses).expect("serialize responses"));
}
