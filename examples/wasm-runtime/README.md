# Writ WASM runtime sample

From the repository root, install `wasm-pack` once, then build the Writ module and WASM package:

```console
cargo install wasm-pack
cargo run -p writ-cli -- compile examples/wasm-runtime/sample.writ --output examples/wasm-runtime/sample.writc
wasm-pack build writ-runtime-wasm --target web --out-dir ../examples/wasm-runtime/pkg
python -m http.server 8000 --directory examples/wasm-runtime
```

Open <http://localhost:8000>. Do not open `index.html` with `file://`; browsers block module and `.writc` fetches in that mode.

The generated `pkg/` directory and `sample.writc` are build outputs and are intentionally not committed. The page demonstrates a synchronous callback response, the Writ log callback, a bounded `requestAnimationFrame` loop, and a deferred request resolved by `setTimeout`.
