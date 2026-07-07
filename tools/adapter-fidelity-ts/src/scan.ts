{
  "name": "@loom-fidelity/adapter-fidelity-ts",
  "version": "1.0.0",
  "description": "Structural fidelity scanner for loom adapter source code",
  "main": "dist/index.js",
  "bin": {
    "adapter-fidelity": "dist/index.js"
  },
  "scripts": {
    "build": "tsc",
    "start": "node dist/index.js"
  },
  "dependencies": {
    "ts-morph": "^21.0.1"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.3.0"
  }
}
