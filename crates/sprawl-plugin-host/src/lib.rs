use anyhow::Result;
use std::path::Path;
use wasmtime::component::*;
use wasmtime::*;
use wasmtime_wasi::{
    DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView,
};

// Sync bindgen — async: true was removed in wasmtime 46.x.
// The plugin interface is synchronous; async dispatch is done at the Archaeologist layer.
wasmtime::component::bindgen!({
    path: "../../plugins/wit/stack-detector.wit",
    world: "stack-detector-plugin",
});

pub use exports::sprawl::stack_detector::detector::{
    Dependency, ReproducibilityVerdict, StackInfo,
};

pub mod verify;
pub use verify::{Ed25519Verifier, PluginManifest};

pub struct HostState {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

pub struct PluginHost {
    engine: Engine,
    linker: wasmtime::component::Linker<HostState>,
    allow_unsigned: bool,
    verifier: Option<verify::Ed25519Verifier>,
}

pub struct LoadedPlugin {
    pub component: Component,
    pub name: String,
}

impl PluginHost {
    pub fn new(allow_unsigned: bool, public_key_bytes: Option<&[u8; ed25519_dalek::PUBLIC_KEY_LENGTH]>) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // No async_support — we use the sync linker and sync instantiation.
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let mut linker = wasmtime::component::Linker::new(&engine);

        // Sync WASI linker (matches non-async store).
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        
        let verifier = if let Some(bytes) = public_key_bytes {
            Some(verify::Ed25519Verifier::new(bytes)?)
        } else {
            None
        };

        Ok(Self { engine, linker, allow_unsigned, verifier })
    }

    pub fn load_plugin(&self, path: &Path, name: &str, manifest: Option<&verify::PluginManifest>) -> Result<LoadedPlugin> {
        if let Some(manifest) = manifest {
            if let Some(verifier) = &self.verifier {
                if !verifier.verify_file(path, &manifest.signature_hex)? {
                    return Err(anyhow::anyhow!("Signature verification failed for plugin: {}", name));
                }
            } else if !self.allow_unsigned {
                 return Err(anyhow::anyhow!("No verifier configured and unsigned plugins are not allowed"));
            }
        } else if !self.allow_unsigned {
            return Err(anyhow::anyhow!("Plugin is unsigned and --allow-unsigned was not provided"));
        }
        let component = Component::from_file(&self.engine, path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load plugin component from {}: {}",
                path.display(),
                e
            )
        })?;

        Ok(LoadedPlugin {
            component,
            name: name.to_string(),
        })
    }

    pub fn detect_stack(
        &self,
        plugin: &LoadedPlugin,
        project_root: &Path,
    ) -> Result<Option<exports::sprawl::stack_detector::detector::StackInfo>> {
        let mut wasi = WasiCtxBuilder::new();
        // Give plugin read-only access to the project root mapped at "/project"
        wasi.preopened_dir(project_root, "/project", DirPerms::READ, FilePerms::READ)?;

        let state = HostState {
            wasi_ctx: wasi.build(),
            resource_table: ResourceTable::new(),
        };

        let mut store = Store::new(&self.engine, state);
        // Fuel limit — equivalent to a few seconds of execution
        store.set_fuel(1_000_000_000)?;

        let bindings =
            StackDetectorPlugin::instantiate(&mut store, &plugin.component, &self.linker)?;

        let result = bindings
            .sprawl_stack_detector_detector()
            .call_detect(&mut store, "/project")?;

        Ok(result)
    }
}

pub struct PluginRegistry {
    plugins: Vec<LoadedPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn register(&mut self, plugin: LoadedPlugin) {
        self.plugins.push(plugin);
    }

    /// Run detection across all registered plugins.
    pub fn run_discovery(
        &self,
        host: &PluginHost,
        project_root: &Path,
    ) -> Result<Vec<exports::sprawl::stack_detector::detector::StackInfo>> {
        let mut results = Vec::new();

        for plugin in &self.plugins {
            match host.detect_stack(plugin, project_root) {
                Ok(Some(info)) => {
                    tracing::info!("Plugin {} successfully detected stack", plugin.name);
                    results.push(info);
                }
                Ok(None) => {
                    tracing::debug!("Plugin {} returned no match", plugin.name);
                }
                Err(e) => {
                    tracing::warn!("Plugin {} failed or crashed: {}", plugin.name, e);
                }
            }
        }

        Ok(results)
    }
}
