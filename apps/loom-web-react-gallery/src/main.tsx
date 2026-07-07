[package]
name = "loom-web-react-gallery"
version = "0.1.0"
edition = "2021"

[dependencies]
loom-core = { path = "../../core" }
loom-web-react = { path = "../../loom-web-react" }
loom-ffi = { path = "../../loom-ffi" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
