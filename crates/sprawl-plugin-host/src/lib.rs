use anyhow::{Context, Result};
use std::path::Path;
use wasmtime::*;
use wasmtime::component::*;
use wasmtime_wasi::{WasiCtx, WasiView, WasiCtxBuilder, ResourceTable};
use wasmtime_wasi::{DirPerms, FilePerms};

wasmtime::component::bindgen!({
    path: "../../plugins/wit/stack-detector.wit",
    world: "stack-detector-plugin",
    async: true,
});

pub struct HostState {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

pub struct PluginHost {
    engine: Engine,
    linker: wasmtime::component::Linker<HostState>,
}

pub struct LoadedPlugin {
    pub component: Component,
    pub name: String,
}

impl PluginHost {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        
        // Add WASI to the linker
        wasmtime_wasi::add_to_linker_async(&mut linker)?;
        
        Ok(Self { engine, linker })
    }

    pub fn load_plugin(&self, path: &Path, name: &str) -> Result<LoadedPlugin> {
        let component = Component::from_file(&self.engine, path)
            .with_context(|| format!("Failed to load plugin component from {}", path.display()))?;
            
        Ok(LoadedPlugin {
            component,
            name: name.to_string(),
        })
    }

    pub async fn detect_stack(&self, plugin: &LoadedPlugin, project_root: &Path) -> Result<Option<exports::sprawl::stack_detector::detector::StackInfo>> {
        let mut wasi = WasiCtxBuilder::new();
        // Give plugin read-only access mapped to "/project"
        wasi.preopened_dir(project_root, "/project", DirPerms::READ, FilePerms::READ)?;
        
        let state = HostState {
            wasi_ctx: wasi.build(),
            resource_table: ResourceTable::new(),
        };
        
        let mut store = Store::new(&self.engine, state);
        // Fuel equivalent to a few seconds of execution
        store.set_fuel(1_000_000_000)?;
        
        let bindings = StackDetectorPlugin::instantiate_async(&mut store, &plugin.component, &self.linker).await?;
        
        // Execute the detect method from the exported detector interface
        let result = bindings.sprawl_stack_detector_detector().call_detect(&mut store, "/project").await?;
        
        Ok(result)
    }
}

pub struct PluginRegistry {
    plugins: Vec<LoadedPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }
    
    pub fn register(&mut self, plugin: LoadedPlugin) {
        self.plugins.push(plugin);
    }
    
    pub async fn run_discovery(&self, host: &PluginHost, project_root: &Path) -> Result<Vec<exports::sprawl::stack_detector::detector::StackInfo>> {
        let mut results = Vec::new();
        
        for plugin in &self.plugins {
            match host.detect_stack(plugin, project_root).await {
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
