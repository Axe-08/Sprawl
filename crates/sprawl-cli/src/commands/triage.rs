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
    let mut sweeper = SweeperEngine::new();

    match &args.command {
        TriageCommand::List => {
            // For MVP/M18, this lists dummy/stub data similar to SweeperInboxState in TUI.
            if is_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "items": [
                            {"project": "old-api/node_modules", "last_seen": "45d ago", "size": "387MB", "status": "[X] nuke-eligible"},
                            {"project": "web-app/dist", "last_seen": "12d ago", "size": "52MB", "status": "[?] ambiguous"}
                        ]
                    })
                );
            } else {
                println!("{:<22} {:<12} {:<9} {}", "PROJECT", "LAST SEEN", "SIZE", "STATUS");
                println!("{:<22} {:<12} {:<9} {}", "old-api/node_modules", "45d ago", "387MB", "[X] nuke-eligible");
                println!("{:<22} {:<12} {:<9} {}", "web-app/dist", "12d ago", "52MB", "[?] ambiguous");
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
            
            let path = PathBuf::from(project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }

            let mut item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern: "mock".into(),
                size_bytes: 0,
                idle_days: 0,
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

            let path = PathBuf::from(project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }

            let mut item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern: "mock".into(),
                size_bytes: 0,
                idle_days: 0,
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

            let path = PathBuf::from(project);
            if !path.exists() {
                if !is_json { println!("Error: path does not exist."); }
                std::process::exit(1);
            }

            let mut item = TriageItem {
                project_id: ProjectId("1".into()),
                project_root: path.clone(),
                target_path: path.clone(),
                matched_pattern: "mock".into(),
                size_bytes: 0,
                idle_days: 0,
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
