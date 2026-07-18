use crate::{Playground, run_source};
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Playground)]
pub struct WasmPlayground {
    playground: Playground,
}

#[wasm_bindgen(js_class = Playground)]
impl WasmPlayground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            playground: Playground::new(),
        }
    }

    #[wasm_bindgen(js_name = setWriteCallback)]
    pub fn set_write_callback(&self, callback: Function) {
        self.playground.set_write_callback(callback);
    }

    #[wasm_bindgen(js_name = sendReadData)]
    pub fn send_read_data(&self, bytes: &[u8]) {
        self.playground.send_read_data(bytes);
    }

    #[wasm_bindgen(js_name = setConfig)]
    pub fn set_config(&mut self, config: &str) -> String {
        match self.playground.set_config(config.as_bytes()) {
            Ok(()) => success_json(),
            Err(error) => error_json(error),
        }
    }

    #[wasm_bindgen(js_name = setSource)]
    pub fn set_source(&mut self, path: &str, source: &str) -> String {
        match self.playground.set_source(path, source.as_bytes()) {
            Ok(()) => success_json(),
            Err(error) => error_json(error),
        }
    }

    #[wasm_bindgen(js_name = check)]
    pub fn check(&mut self) -> String {
        let result = self.playground.check();
        serde_json::json!({
            "is_valid": result.is_valid,
            "diagnostics": result.diagnostics,
        })
        .to_string()
    }

    #[wasm_bindgen(js_name = compile)]
    pub fn compile(&mut self) -> String {
        match self.playground.compile() {
            Ok(()) => success_json(),
            Err(error) => error_json(error),
        }
    }

    #[wasm_bindgen(js_name = run)]
    pub fn run(&mut self, args_json: &str) -> String {
        let args = match serde_json::from_str::<Vec<String>>(args_json) {
            Ok(args) => args.into_iter().map(String::into_bytes).collect::<Vec<_>>(),
            Err(error) => return error_json(error),
        };
        match self.playground.run(args.as_slice()) {
            Ok(exit_code) => serde_json::json!({
                "exit_code": exit_code,
                "output": String::from_utf8_lossy(self.playground.take_output().as_slice()),
                "error": null,
            })
            .to_string(),
            Err(error) => error_json(error),
        }
    }
}

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

fn success_json() -> String {
    serde_json::json!({ "ok": true }).to_string()
}

fn error_json(error: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "error": error.to_string() }).to_string()
}
