#![no_main]
use libfuzzer_sys::fuzz_target;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use sprawl_plugin_host::PluginHost;

fuzz_target!(|data: &[u8]| {
    // We want to ensure that a maliciously crafted WASM file won't crash the host plugin loader.
    // We'll write the fuzz data to a temporary file and attempt to load it.
    if let Ok(temp_dir) = TempDir::new() {
        let wasm_path = temp_dir.path().join("fuzz.wasm");
        if fs::write(&wasm_path, data).is_ok() {
            if let Ok(host) = PluginHost::new(true, None) {
                // We just want to see if this panics
                let _ = host.load_plugin(&wasm_path, "fuzzer", None);
            }
        }
    }
});
