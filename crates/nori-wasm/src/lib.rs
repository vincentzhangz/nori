//! WASM bindings for compiling Nori source in-process.
//!
//! Build with:
//! ```text
//! wasm-pack build crates/nori-wasm --target bundler --out-dir ../../packages/compiler-wasm/pkg
//! ```
//!
//! On non-`wasm32` targets this crate still compiles (`cargo check -p nori-wasm`)
//! so CI/local hosts can typecheck the bindings without a wasm toolchain.

use wasm_bindgen::prelude::*;

/// Compile Nori source to JavaScript.
///
/// Returns the generated JS string, or a `JsValue` error message on failure.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String, JsValue> {
    compile_with_runtime(source, "@nori/core")
}

/// Compile Nori source using a custom runtime import path.
#[wasm_bindgen(js_name = compileWithRuntime)]
pub fn compile_with_runtime(source: &str, runtime_import: &str) -> Result<String, JsValue> {
    nori::compile_source(
        source,
        nori::CompileOptions {
            filename: "<wasm>.nori".to_string(),
            runtime_import: runtime_import.to_string(),
            ..nori::CompileOptions::default()
        },
    )
    .map(|output| output.code)
    .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_simple_component() {
        let code = compile_with_runtime(
            "const count = $state(0);\nexport default function C() { return <p>{count.value}</p>; }",
            "@nori/core",
        )
        .expect("compile");
        assert!(code.contains("signal(0)"));
        assert!(code.contains("h("));
    }
}
