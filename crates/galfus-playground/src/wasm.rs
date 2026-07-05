use crate::run_source;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_source_wasm(code: &str, args_json: &str) -> String {
    let args = serde_json::from_str::<Vec<String>>(args_json).unwrap_or_default();
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let result = run_source(code, arg_refs.as_slice());

    serde_json::json!({
        "output": result.output,
        "exit_code": result.exit_code,
        "error": result.error,
    })
    .to_string()
}
