use clap::{Args, Subcommand};
use sprawl_core::Result;
use std::path::PathBuf;

#[derive(Args)]
pub struct ProjectArgs {
    #[command(subcommand)]
    pub action: ProjectAction,
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// Register a directory for indexing (defaults to current directory)
    Add {
        /// Path to the project root
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Pause indexing for a directory (sets status to idle, preserves history)
    Remove {
        /// Path to the project root
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Permanently delete from ledger (no history preserved)
        #[arg(long)]
        hard: bool,
    },
    /// List all registered projects
    List,
}

/// Write a project directly to the ledger without daemon (offline fallback).
fn write_project_offline(path: &str) -> Result<String> {
    let ledger_path = sprawl_core::platform::sprawl_data_dir()?.join("ledger.sqlite");
    let conn = sprawl_core::ledger::initialize_db(&ledger_path)
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT OR IGNORE INTO projects (id, root_path, status, last_seen, created_at) VALUES (?1, ?2, 'active', ?3, ?3)",
        rusqlite::params![id, path, now],
    ).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let existing_id: String = conn
        .query_row(
            "SELECT id FROM projects WHERE root_path = ?1",
            rusqlite::params![path],
            |r| r.get(0),
        )
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    Ok(existing_id)
}

fn remove_project_offline(path: &str, hard: bool) -> Result<()> {
    let ledger_path = sprawl_core::platform::sprawl_data_dir()?.join("ledger.sqlite");
    let conn = sprawl_core::ledger::initialize_db(&ledger_path)
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    if hard {
        conn.execute(
            "DELETE FROM projects WHERE root_path = ?1",
            rusqlite::params![path],
        ).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    } else {
        conn.execute(
            "UPDATE projects SET status = 'idle' WHERE root_path = ?1",
            rusqlite::params![path],
        ).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    }
    Ok(())
}

fn list_projects_offline() -> Result<Vec<sprawl_daemon::IpcProjectEntry>> {
    let ledger_path = sprawl_core::platform::sprawl_data_dir()?.join("ledger.sqlite");
    let conn = sprawl_core::ledger::initialize_db(&ledger_path)
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, root_path, status, ecosystem, created_at FROM projects ORDER BY created_at DESC",
        )
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let entries = stmt
        .query_map([], |r| {
            Ok(sprawl_daemon::IpcProjectEntry {
                id: r.get(0)?,
                root_path: r.get(1)?,
                status: r.get(2)?,
                ecosystem: r.get(3)?,
                created_at: r.get(4)?,
            })
        })
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    Ok(entries.flatten().collect())
}

fn print_projects(projects: &[sprawl_daemon::IpcProjectEntry]) {
    if projects.is_empty() {
        println!("No projects registered. Use `sprawl project add <path>` to add one.");
        return;
    }

    println!(
        "{:<36}  {:<10}  {}",
        "ID", "STATUS", "PATH"
    );
    println!("{}", "─".repeat(80));
    for p in projects {
        let short_id = &p.id[..8];
        let status_colored = match p.status.as_str() {
            "active" => format!("\x1b[32m{:<10}\x1b[0m", p.status),
            "idle" => format!("\x1b[33m{:<10}\x1b[0m", p.status),
            _ => format!("{:<10}", p.status),
        };
        println!("{:<36}  {}  {}", short_id, status_colored, p.root_path);
    }
}

pub async fn handle(args: &ProjectArgs, _is_json: bool) -> Result<()> {
    // Try IPC first; fall back to direct ledger access if daemon isn't running
    let client = sprawl_daemon::IpcClient::new().ok();

    match &args.action {
        ProjectAction::Add { path } => {
            let abs = path
                .canonicalize()
                .unwrap_or_else(|_| path.to_path_buf());
            let abs_str = abs.to_string_lossy().into_owned();

            // Try daemon first, then offline
            let id = if let Some(c) = client {
                match c.send_request(&sprawl_daemon::IpcRequest::RegisterProject { path: abs_str.clone() }).await {
                    Ok(sprawl_daemon::IpcResponse::ProjectRegistered { id }) => id,
                    Ok(sprawl_daemon::IpcResponse::Error(e)) => {
                        return Err(sprawl_core::SprawlError::Other(e));
                    }
                    _ => write_project_offline(&abs_str)?,
                }
            } else {
                write_project_offline(&abs_str)?
            };

            println!("✓ Registered project");
            println!("  Path: {}", abs_str);
            println!("  ID:   {}", id);
            println!();
            println!("Next steps:");
            println!("  sprawl daemon start    — start the background daemon");
            println!("  sprawl index --start   — trigger indexing and watch progress");
        }

        ProjectAction::Remove { path, hard } => {
            let abs = path
                .canonicalize()
                .unwrap_or_else(|_| path.to_path_buf());
            let abs_str = abs.to_string_lossy().into_owned();

            if let Some(c) = client {
                match c.send_request(&sprawl_daemon::IpcRequest::UnregisterProject {
                    path: abs_str.clone(),
                    hard: *hard,
                }).await {
                    Ok(sprawl_daemon::IpcResponse::Ok) => {}
                    Ok(sprawl_daemon::IpcResponse::Error(e)) => {
                        return Err(sprawl_core::SprawlError::Other(e));
                    }
                    _ => remove_project_offline(&abs_str, *hard)?,
                }
            } else {
                remove_project_offline(&abs_str, *hard)?;
            }

            if *hard {
                println!("✓ Permanently deleted project from ledger: {}", abs_str);
            } else {
                println!("✓ Paused indexing for: {}", abs_str);
                println!("  Status set to 'idle'. Use `sprawl project add {}` to re-activate.", abs_str);
            }
        }

        ProjectAction::List => {
            let projects = if let Some(c) = client {
                match c.send_request(&sprawl_daemon::IpcRequest::ListProjects).await {
                    Ok(sprawl_daemon::IpcResponse::Projects(p)) => p,
                    _ => list_projects_offline()?,
                }
            } else {
                list_projects_offline()?
            };

            print_projects(&projects);
        }
    }

    Ok(())
}
