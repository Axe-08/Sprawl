// Hardcoded defaults for MVP per OQ-04 resolution

pub const GLOBAL_DEFAULTS_TOML: &str = r#"
[[sweep_target]]
path = "node_modules"
condition = "idle_days > 14"
action = "nuke_safe"

[[sweep_target]]
path = "target"
condition = "idle_days > 14"
action = "nuke_safe"

[[sweep_target]]
path = "__pycache__"
condition = "idle_days > 7"
action = "nuke_safe"
"#;

pub const WEB_DEV_TOML: &str = r#"
[[sweep_target]]
path = "node_modules"
condition = "idle_days > 14"
action = "nuke_safe"

[[sweep_target]]
path = ".next"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "dist"
condition = "idle_days > 14"
action = "nuke_safe"

[[sweep_target]]
path = "build"
condition = "idle_days > 14"
action = "nuke_safe"

[[sweep_target]]
path = ".cache"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = ".turbo"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = ".parcel-cache"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "coverage"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "playwright-report"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "cypress/videos"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "cypress/screenshots"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = ".vite"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "out"
condition = "idle_days > 14"
action = "nuke_safe"

[[noise_pattern]]
pattern = "regex:^[a-f0-9]{8,}\\.chunk\\.(js|css)$"
description = "Webpack/Vite chunk-hash filenames"

[[noise_pattern]]
pattern = "regex:\\.map$"
description = "Source map files"
"#;

pub const ML_ENGINEER_TOML: &str = r#"
[[sweep_target]]
path = "__pycache__"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = ".ipynb_checkpoints"
condition = "idle_days > 7"
action = "nuke_safe"

[[sweep_target]]
path = "wandb"
condition = "idle_days > 14"
action = "archive"

[[sweep_target]]
path = "mlruns"
condition = "idle_days > 14"
action = "archive"

[[sweep_target]]
path = "lightning_logs"
condition = "idle_days > 14"
action = "archive"

[[sweep_target]]
path = ".venv"
condition = "idle_days > 30"
action = "archive"

[[noise_pattern]]
pattern = "regex:^[a-f0-9]{8,}$"
description = "Checkpoint/run-ID hex directory names"
"#;

pub fn get_preset_toml(persona: &str) -> Option<&'static str> {
    match persona {
        "global_defaults" => Some(GLOBAL_DEFAULTS_TOML),
        "web-dev" => Some(WEB_DEV_TOML),
        "ml-engineer" => Some(ML_ENGINEER_TOML),
        _ => None,
    }
}
