use clap::Args;
use sprawl_core::Result;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Args)]
pub struct ResurrectArgs {
    /// Path of the archived project to resurrect
    pub project_path: String,
}

pub fn handle(args: &ResurrectArgs, is_json: bool) -> Result<()> {
    let engine = sprawl_sweeper::engine::SweeperEngine::new();

    if args.project_path.is_empty() {
        return Err(sprawl_core::SprawlError::Other(
            "Project path cannot be empty".into(),
        ));
    }

    let project_path = PathBuf::from(&args.project_path);
    let absolute_path = if project_path.is_absolute() {
        project_path.clone()
    } else {
        std::env::current_dir().unwrap_or_default().join(project_path)
    };

    let target_path = absolute_path.join("node_modules"); // Simulating the swept dir
    let project_name = absolute_path.file_name().unwrap_or_default().to_string_lossy();
    let archive_path = PathBuf::from(format!(
        "/tmp/sprawl_archive/{}/node_modules",
        project_name
    ));

    // Validate the archive actually exists before proceeding
    if !archive_path.exists() {
        let ledger_has_project = {
            let data_dir = sprawl_core::platform::sprawl_data_dir()?;
            let ledger_path = data_dir.join("ledger.sqlite");
            if let Ok(conn) = rusqlite::Connection::open(&ledger_path) {
                conn.query_row(
                    "SELECT 1 FROM projects WHERE root_path = ?1",
                    [absolute_path.to_string_lossy().as_ref()],
                    |_| Ok(true),
                ).unwrap_or(false)
            } else {
                false
            }
        };

        if !ledger_has_project {
            if is_json {
                println!("{}", serde_json::json!({
                    "status": "error",
                    "message": format!("Project '{}' not found in ledger or archive. Has it been swept by Sprawl?", args.project_path)
                }));
            } else {
                eprintln!("Error: Project '{}' not found in ledger or archive. Has it been swept by Sprawl?", args.project_path);
            }
            std::process::exit(4);
        }
    }

    // Restore the files (mocked or real based on backend)
    match engine.restore(&target_path, &archive_path) {
        Ok(_) => {
            // Write resurrection-kit.md
            let kit_path = absolute_path.join("resurrection-kit.md");
            if let Ok(mut file) = File::create(&kit_path) {
                let content = format!(
                    "# Resurrection Kit: {}\n\n\
                    This project was resurrected from cold storage by Sprawl.\n\n\
                    ## What happened?\n\
                    - Archived dependencies have been restored from: `{}`\n\
                    - You may need to run package manager installations to ensure binaries are built.\n\n\
                    ## Next Steps\n\
                    Run the command copied to your clipboard to bootstrap your environment.\n",
                    project_name, archive_path.display()
                );
                let _ = file.write_all(content.as_bytes());
            }

            // Copy to clipboard
            let cmd = format!("cd {} && echo 'Ready to build'", absolute_path.display());
            let mut copied_to_clipboard = false;
            
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                if ctx.set_text(&cmd).is_ok() {
                    copied_to_clipboard = true;
                }
            }

            if !is_json {
                println!("✅ Project '{}' resurrected successfully.", project_name);
                println!("📄 Generated {}", kit_path.display());
                if copied_to_clipboard {
                    println!("📋 Copied bootstrap command to clipboard!");
                } else {
                    println!("💡 Bootstrap command: {}", cmd);
                }
            }
        }
        Err(e) => {
            return Err(sprawl_core::SprawlError::Other(format!(
                "Resurrect failed: {}",
                e
            )));
        }
    }

    Ok(())
}
