use clap::{Args, Subcommand};
use sprawl_core::Result;
use sprawl_sweeper::{SweeperEngine, TriageAction, TriageItem, NukeEligibility};
use sprawl_sweeper::engine::ProjectId;
use std::path::PathBuf;

#[derive(Args)]
pub struct TriageArgs {
    #[command(subcommand)]
    pub command: TriageCommand,
}

#[derive(Subcommand)]
pub enum TriageCommand {
    /// Show current triage queue
    List,
    /// Nuke with safety-gate re-verify
    Nuke { project: String },
    /// Archive to ~/.sprawl/archive/
    Archive { project: String },
    /// Snooze for 30d
    Snooze { project: String },
}

pub fn handle(args: &TriageArgs, is_json: bool) -> Result<()> {
    let sweeper = SweeperEngine::new();

    match &args.command {
        TriageCommand::List => {
            let data_dir = sprawl_core::platform::sprawl_data_dir()?;
            let ledger_path = data_dir.join("ledger.sqlite");
            
            struct TriageCandidate {
                project: String,
                idle_days: i64,
                size_bytes: u64,
                status: &'static str,
            }
            let mut items = Vec::new();

            let candidate_patterns = ["node_modules", "dist", "target", ".venv", "__pycache__", ".next", "build"];
            if let Ok(conn) = rusqlite::Connection::open(&ledger_path) {
                if let Ok(mut stmt) = conn.prepare("SELECT root_path FROM projects") {
                    let roots: Vec<String> = stmt.query_map([], |r| r.get(0))
                        .map(|m| m.flatten().collect())
                        .unwrap_or_default();
                    for root in roots {
                        for pattern in &candidate_patterns {
                            let candidate = std::path::PathBuf::from(&root).join(pattern);
                            if candidate.exists() {
                                let (size_bytes, idle_days) = get_directory_metadata(&candidate);
                                let status = if idle_days > 30 { "[X] nuke-eligible" } else { "[?] ambiguous" };
                                items.push(TriageCandidate {
                                    project: format!("{}/{}", std::path::PathBuf::from(&root).file_name().unwrap_or_default().to_string_lossy(), pattern),
                                    idle_days,
                                    size_bytes,
                                    status,
                                });
                            }
                        }
                    }
                }
            }

            if items.is_empty() {
                if is_json {
                    println!("{}", serde_json::json!({"items": []}));
                } else {
                    println!("No triage candidates found. Run `sprawl analyze <dir>` to register projects.");
                }
                return Ok(());
            }

            if is_json {
                println!("{}", serde_json::json!({"items": items.iter().map(|m| serde_json::json!({
                    "project": m.project,
                    "last_seen": format!("{}d ago", m.idle_days),
                    "size": format!("{}MB", m.size_bytes / 1_000_000),
                    "status": m.status
                })).collect::<Vec<_>>()}));
            } else {
                println!("{:<30} {:<12} {:<9} {}", "PROJECT", "LAST SEEN", "SIZE", "STATUS");
                for item in items {
                    println!("{:<30} {:<12} {:<9} {}", item.project, format!("{}d ago", item.idle_days), format!("{}MB", item.size_bytes / 1_000_000), item.status);
                }
            }
        }
        TriageCommand::Nuke { project } => {
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({"action": "nuke", "project": project, "status": "pending"})
                );
            } else {
                println!("Attempting to nuke {}...", project);
            }
            
            let path = PathBuf::from(&project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }
            if let Err(e) = validate_triage_path(&path) {
                if !is_json { println!("Error: {}", e); }
                std::process::exit(1);
            }

            let (size_bytes, idle_days) = get_directory_metadata(&path);
            let matched_pattern = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern,
                size_bytes,
                idle_days,
                nuke_eligibility: NukeEligibility::Eligible,
                recommended_action: TriageAction::NukeSafe,
            };

            match sweeper.nuke(&item, None) {
                Ok(_) => {
                    if !is_json { println!("Successfully nuked {}.", project); }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if !is_json { println!("Failed to nuke: {}", msg); }
                    let msg_lower = msg.to_lowercase();
                    if msg_lower.contains("safety gate") || msg_lower.contains("locked") {
                        std::process::exit(2);
                    } else {
                        std::process::exit(1);
                    }
                }
            }
        }
        TriageCommand::Archive { project } => {
            if is_json {
                println!("{}", serde_json::json!({"action": "archive", "project": project, "status": "pending"}));
            } else {
                println!("Attempting to archive {}...", project);
            }

            let path = PathBuf::from(&project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }
            if let Err(e) = validate_triage_path(&path) {
                if !is_json { println!("Error: {}", e); }
                std::process::exit(1);
            }

            let (size_bytes, idle_days) = get_directory_metadata(&path);
            let matched_pattern = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern,
                size_bytes,
                idle_days,
                nuke_eligibility: NukeEligibility::Eligible,
                recommended_action: TriageAction::Archive,
            };

            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let archive_dir = std::path::PathBuf::from(home).join(".sprawl").join("archive");

            match sweeper.archive(&item, &archive_dir) {
                Ok(_) => {
                    if !is_json { println!("Successfully archived {}.", project); }
                }
                Err(e) => {
                    if !is_json { println!("Failed to archive: {}", e); }
                    std::process::exit(1);
                }
            }
        }
        TriageCommand::Snooze { project } => {
            if is_json {
                println!("{}", serde_json::json!({"action": "snooze", "project": project, "status": "pending"}));
            } else {
                println!("Attempting to snooze {}...", project);
            }

            let path = PathBuf::from(&project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }
            if let Err(e) = validate_triage_path(&path) {
                if !is_json { println!("Error: {}", e); }
                std::process::exit(1);
            }

            let (size_bytes, idle_days) = get_directory_metadata(&path);
            let matched_pattern = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern,
                size_bytes,
                idle_days,
                nuke_eligibility: NukeEligibility::Eligible,
                recommended_action: TriageAction::Snooze,
            };

            match sweeper.snooze(&item, 30) {
                Ok(_) => {
                    if !is_json { println!("Successfully snoozed {}.", project); }
                }
                Err(e) => {
                    if !is_json { println!("Failed to snooze: {}", e); }
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn get_directory_metadata(path: &std::path::Path) -> (u64, i64) {
    let mut total_size = 0;
    let mut latest_mtime = std::time::SystemTime::UNIX_EPOCH;

    if path.is_file() {
        if let Ok(metadata) = path.metadata() {
            total_size = metadata.len();
            if let Ok(mtime) = metadata.modified() {
                latest_mtime = mtime;
            }
        }
    } else if path.is_dir() {
        for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                    if let Ok(mtime) = metadata.modified() {
                        if mtime > latest_mtime {
                            latest_mtime = mtime;
                        }
                    }
                }
            }
        }
    }

    let idle_days = if latest_mtime != std::time::SystemTime::UNIX_EPOCH {
        if let Ok(duration) = std::time::SystemTime::now().duration_since(latest_mtime) {
            (duration.as_secs() / 86400) as i64
        } else {
            0
        }
    } else {
        0
    };

    (total_size, idle_days)
}

fn validate_triage_path(path: &std::path::Path) -> sprawl_core::Result<()> {
    let candidate_patterns = ["node_modules", "dist", "target", ".venv", "__pycache__", ".next", "build"];
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    if !candidate_patterns.contains(&file_name.as_ref()) {
        return Err(sprawl_core::SprawlError::Other(format!("Path '{}' does not match any noisy directory pattern", path.display())));
    }
    
    // Check if parent is a registered project
    if let Some(parent) = path.parent() {
        if let Ok(ledger_path) = sprawl_core::platform::sprawl_data_dir().map(|d| d.join("ledger.sqlite")) {
            if let Ok(conn) = sprawl_core::ledger::initialize_db(&ledger_path) {
                let parent_str = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf()).to_string_lossy().to_string();
                let mut stmt = conn.prepare("SELECT count(*) FROM projects WHERE root_path = ?")
                    .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                let count: i64 = stmt.query_row([&parent_str], |r| r.get(0)).unwrap_or(0);
                if count > 0 {
                    return Ok(());
                }
            }
        }
    }
    Err(sprawl_core::SprawlError::Other(format!("Path '{}' is not within a registered project", path.display())))
}

