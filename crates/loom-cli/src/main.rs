//! loom — the CLI over the store: load | validate | inventory | query
//! (if-cli). JSON out on every subcommand.

use clap::{Parser, Subcommand};
use loom_graph::sparql::SparqlBuilder;
use loom_graph::{catalog_inventory, load_bundle, BundleLoadRequest, LoomStore, NamedGraph};

#[derive(Parser)]
#[command(name = "loom", about = "Design-system graph CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Load a bundle through the SHACL gate into its named graph.
    Load {
        /// Path to the bundle document (JSON-LD against the pinned context).
        bundle_path: String,
        /// Artifact class: tokens | machines | reification | motion | mode-contracts.
        #[arg(long)]
        artifact_class: String,
        #[arg(long, default_value = "local")]
        source: String,
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },
    /// Validate a bundle against shapes/ without touching the store.
    Validate {
        bundle_path: String,
        #[arg(long)]
        artifact_class: String,
    },
    /// Project the catalog inventory (rm-catalog-inventory).
    Inventory,
    /// Blast radius of a token: every component whose tokens bind it.
    Query {
        /// Token IRI, e.g. http://loom.dev/token/surface.interactive
        #[arg(long)]
        blast_radius: String,
    },
}

const BLAST_RADIUS_QUERY: &str = include_str!("../../loom-graph/queries/blast_radius.sparql");

fn main() {
    let cli = Cli::parse();
    let store = LoomStore::new().expect("in-memory store opens");
    let outcome = run(&cli.command, &store);
    match outcome {
        Ok(json) => println!("{json}"),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}

fn run(command: &Command, store: &LoomStore) -> Result<String, String> {
    match command {
        Command::Load {
            bundle_path,
            artifact_class,
            source,
            version,
        } => {
            let request = BundleLoadRequest {
                bundle_path: bundle_path.clone(),
                artifact_class: artifact_class.clone(),
                source: source.clone(),
                version: version.clone(),
            };
            let outcome = load_bundle(store, &request).map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&outcome).map_err(|e| e.to_string())
        }
        Command::Validate {
            bundle_path,
            artifact_class,
        } => {
            let graph = NamedGraph::from_artifact_class(artifact_class)
                .ok_or_else(|| format!("unknown artifact class '{artifact_class}'"))?;
            let raw = std::fs::read_to_string(bundle_path).map_err(|e| e.to_string())?;
            let quads = loom_graph::bundle::load_gate::parse_against_pinned_context(&raw, &graph)
                .map_err(|e| e.to_string())?;
            let shapes = loom_graph::shacl::ShapeSet::for_artifact_class(artifact_class)
                .map_err(|e| e.to_string())?;
            let violations = shapes.validate(&quads).map_err(|e| e.to_string())?;
            let wire: Vec<String> = violations.iter().map(|v| v.wire()).collect();
            serde_json::to_string_pretty(&serde_json::json!({
                "conformant": wire.is_empty(),
                "violations": wire,
            }))
            .map_err(|e| e.to_string())
        }
        Command::Inventory => {
            let inventory = catalog_inventory(store).map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&inventory).map_err(|e| e.to_string())
        }
        Command::Query { blast_radius } => {
            let query = SparqlBuilder::from_query_file(BLAST_RADIUS_QUERY)
                .bind_graph("graph", &NamedGraph::Tokens)
                .bind_iri("token", blast_radius)
                .build();
            #[derive(serde::Deserialize, serde::Serialize)]
            struct Row {
                component: String,
            }
            let rows: Vec<Row> = store.fetch_rows_bound(&query).map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())
        }
    }
}
