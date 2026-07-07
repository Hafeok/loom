//! SHACL validation on load (d-dual-encoding, p-fail-closed-load).
//!
//! A deliberately small SHACL subset — `sh:targetClass`, `sh:property` with
//! `sh:path`, `sh:minCount`, `sh:maxCount`, `sh:minLength`, `sh:pattern`,
//! `sh:nodeKind` — evaluated by pure Rust over parsed quads. The shapes under
//! `shapes/` are the authoritative per-artifact-class assertions, held
//! equivalent to loom-spec's JSON Schemas by the encoding-neutral vectors.

use std::collections::HashMap;

use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{NamedOrBlankNode, Quad, Term};
use regex::Regex;
use thiserror::Error;

/// The shapes files, pinned at compile time, keyed by artifact class.
pub const SHAPES: [(&str, &str); 5] = [
    ("tokens", include_str!("../shapes/tokens.ttl")),
    ("machines", include_str!("../shapes/machines.ttl")),
    ("reification", include_str!("../shapes/reification.ttl")),
    ("motion", include_str!("../shapes/motion.ttl")),
    ("mode-contracts", include_str!("../shapes/mode-contracts.ttl")),
];

#[derive(Error, Debug)]
pub enum ShaclError {
    #[error("no shapes registered for artifact class '{0}'")]
    UnknownArtifactClass(String),
    #[error("shapes file failed to parse: {0}")]
    ShapesParse(String),
    #[error("invalid sh:pattern '{0}': {1}")]
    Pattern(String, regex::Error),
}

/// One violation of a shape constraint, reported fail-closed by the load gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaclViolation {
    pub focus: String,
    pub path: String,
    pub component: String,
    pub message: String,
}

impl ShaclViolation {
    /// The wire form carried by `ev-bundle-rejected.shacl-violations`.
    pub fn wire(&self) -> String {
        format!(
            "{} on <{}> (focus {}): {}",
            self.component, self.path, self.focus, self.message
        )
    }
}

#[derive(Debug, Clone, Default)]
struct PropertyShape {
    path: String,
    min_count: Option<u64>,
    max_count: Option<u64>,
    min_length: Option<u64>,
    pattern: Option<String>,
    node_kind: Option<String>,
}

#[derive(Debug, Clone)]
struct NodeShape {
    target_class: String,
    properties: Vec<PropertyShape>,
}

/// The parsed shapes for one artifact class.
#[derive(Debug, Clone)]
pub struct ShapeSet {
    shapes: Vec<NodeShape>,
}

fn sh(term: &str) -> String {
    format!("{}{term}", loom_ontology::SHACL_NS)
}

fn rdf_type() -> String {
    format!("{}type", loom_ontology::RDF_NS)
}

fn subject_key(s: &NamedOrBlankNode) -> String {
    match s {
        NamedOrBlankNode::NamedNode(n) => n.as_str().to_string(),
        NamedOrBlankNode::BlankNode(b) => format!("_:{}", b.as_str()),
    }
}

fn term_key(t: &Term) -> String {
    match t {
        Term::NamedNode(n) => n.as_str().to_string(),
        Term::BlankNode(b) => format!("_:{}", b.as_str()),
        Term::Literal(l) => l.value().to_string(),
    }
}

impl ShapeSet {
    /// Parse the pinned shapes file for `artifact_class`.
    pub fn for_artifact_class(artifact_class: &str) -> Result<Self, ShaclError> {
        let ttl = SHAPES
            .iter()
            .find(|(class, _)| *class == artifact_class)
            .map(|(_, ttl)| *ttl)
            .ok_or_else(|| ShaclError::UnknownArtifactClass(artifact_class.to_string()))?;
        Self::from_turtle(ttl)
    }

    /// Parse shapes from Turtle (SHACL vocabulary subset).
    pub fn from_turtle(ttl: &str) -> Result<Self, ShaclError> {
        let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::Turtle)
            .for_slice(ttl.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ShaclError::ShapesParse(e.to_string()))?;

        // Index: subject -> predicate -> objects
        let mut index: HashMap<String, HashMap<String, Vec<Term>>> = HashMap::new();
        for q in &quads {
            index
                .entry(subject_key(&q.subject))
                .or_default()
                .entry(q.predicate.as_str().to_string())
                .or_default()
                .push(q.object.clone());
        }

        let node_shape_class = sh("NodeShape");
        let mut shapes = Vec::new();
        for (subject, props) in &index {
            let is_node_shape = props
                .get(&rdf_type())
                .map(|types| types.iter().any(|t| term_key(t) == node_shape_class))
                .unwrap_or(false);
            if !is_node_shape {
                continue;
            }
            let Some(target_class) = props
                .get(&sh("targetClass"))
                .and_then(|v| v.first())
                .map(term_key)
            else {
                return Err(ShaclError::ShapesParse(format!(
                    "node shape {subject} has no sh:targetClass"
                )));
            };
            let mut properties = Vec::new();
            for prop_ref in props.get(&sh("property")).into_iter().flatten() {
                let prop_key = term_key(prop_ref);
                let Some(prop_props) = index.get(&prop_key) else {
                    continue;
                };
                let get_one = |pred: &str| prop_props.get(&sh(pred)).and_then(|v| v.first());
                let get_u64 = |pred: &str| {
                    get_one(pred).and_then(|t| match t {
                        Term::Literal(l) => l.value().parse::<u64>().ok(),
                        _ => None,
                    })
                };
                let Some(path) = get_one("path").map(term_key) else {
                    return Err(ShaclError::ShapesParse(format!(
                        "property shape {prop_key} has no sh:path"
                    )));
                };
                properties.push(PropertyShape {
                    path,
                    min_count: get_u64("minCount"),
                    max_count: get_u64("maxCount"),
                    min_length: get_u64("minLength"),
                    pattern: get_one("pattern").map(|t| match t {
                        Term::Literal(l) => l.value().to_string(),
                        other => term_key(other),
                    }),
                    node_kind: get_one("nodeKind").map(term_key),
                });
            }
            shapes.push(NodeShape {
                target_class,
                properties,
            });
        }
        Ok(Self { shapes })
    }

    /// Validate parsed bundle quads. Returns every violation found; an empty
    /// vector means the bundle is conformant and may enter the store.
    pub fn validate(&self, data: &[Quad]) -> Result<Vec<ShaclViolation>, ShaclError> {
        // Index data: subject -> predicate -> objects
        let mut index: HashMap<String, HashMap<String, Vec<Term>>> = HashMap::new();
        for q in data {
            index
                .entry(subject_key(&q.subject))
                .or_default()
                .entry(q.predicate.as_str().to_string())
                .or_default()
                .push(q.object.clone());
        }

        let mut violations = Vec::new();
        for shape in &self.shapes {
            for (focus, props) in &index {
                let targeted = props
                    .get(&rdf_type())
                    .map(|types| types.iter().any(|t| term_key(t) == shape.target_class))
                    .unwrap_or(false);
                if !targeted {
                    continue;
                }
                for prop in &shape.properties {
                    let empty = Vec::new();
                    let values = props.get(&prop.path).unwrap_or(&empty);
                    let count = values.len() as u64;
                    if let Some(min) = prop.min_count {
                        if count < min {
                            violations.push(ShaclViolation {
                                focus: focus.clone(),
                                path: prop.path.clone(),
                                component: "sh:MinCountConstraintComponent".into(),
                                message: format!("expected at least {min} value(s), found {count}"),
                            });
                        }
                    }
                    if let Some(max) = prop.max_count {
                        if count > max {
                            violations.push(ShaclViolation {
                                focus: focus.clone(),
                                path: prop.path.clone(),
                                component: "sh:MaxCountConstraintComponent".into(),
                                message: format!("expected at most {max} value(s), found {count}"),
                            });
                        }
                    }
                    for value in values {
                        if let Some(min_len) = prop.min_length {
                            let lex = match value {
                                Term::Literal(l) => l.value().to_string(),
                                other => term_key(other),
                            };
                            if (lex.chars().count() as u64) < min_len {
                                violations.push(ShaclViolation {
                                    focus: focus.clone(),
                                    path: prop.path.clone(),
                                    component: "sh:MinLengthConstraintComponent".into(),
                                    message: format!(
                                        "value '{lex}' is shorter than minLength {min_len}"
                                    ),
                                });
                            }
                        }
                        if let Some(pattern) = &prop.pattern {
                            let re = Regex::new(pattern)
                                .map_err(|e| ShaclError::Pattern(pattern.clone(), e))?;
                            let lex = match value {
                                Term::Literal(l) => l.value().to_string(),
                                other => term_key(other),
                            };
                            if !re.is_match(&lex) {
                                violations.push(ShaclViolation {
                                    focus: focus.clone(),
                                    path: prop.path.clone(),
                                    component: "sh:PatternConstraintComponent".into(),
                                    message: format!("value '{lex}' does not match /{pattern}/"),
                                });
                            }
                        }
                        if let Some(kind) = &prop.node_kind {
                            let ok = match kind.as_str() {
                                k if k == sh("IRI") => matches!(value, Term::NamedNode(_)),
                                k if k == sh("Literal") => matches!(value, Term::Literal(_)),
                                k if k == sh("BlankNodeOrIRI") => {
                                    matches!(value, Term::NamedNode(_) | Term::BlankNode(_))
                                }
                                _ => true,
                            };
                            if !ok {
                                violations.push(ShaclViolation {
                                    focus: focus.clone(),
                                    path: prop.path.clone(),
                                    component: "sh:NodeKindConstraintComponent".into(),
                                    message: format!("value {} has wrong node kind", term_key(value)),
                                });
                            }
                        }
                    }
                }
            }
        }
        violations.sort_by(|a, b| a.wire().cmp(&b.wire()));
        Ok(violations)
    }
}
