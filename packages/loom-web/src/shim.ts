{
  "name": "@loom/web",
  "version": "0.1.0",
  "description": "The web mode for loom — compiled loom-wasm runtime plus DOM shim.",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "build:wasm": "cargo build --target wasm32-unknown-unknown --release && wasm-bindgen target/wasm32-unknown-unknown/release/loom_wasm.wasm --out-dir dist/wasm --target web"
  },
  "module": "dist/index.js",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "types": "./dist/index.d.ts"
    }
  },
  "dependencies": {
    "loom-wasm": "workspace:*"
  }
}
