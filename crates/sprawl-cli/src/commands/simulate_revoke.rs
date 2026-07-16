use clap::Args;
use sprawl_core::Result;
use uuid::Uuid;

#[derive(Args)]
pub struct SimulateRevokeArgs {
    /// The UUID of the secret to simulate revoking
    #[arg(short, long)]
    pub key: String,
}

pub fn handle(args: &SimulateRevokeArgs, is_json: bool) -> Result<()> {
    let _secret_id = match Uuid::parse_str(&args.key) {
        Ok(id) => id,
        Err(_) => {
            if is_json {
                println!("{}", serde_json::json!({"status": "error", "message": "Invalid UUID format"}));
            } else {
                println!("Error: Invalid UUID format");
            }
            std::process::exit(1);
        }
    };

    let data_dir = sprawl_core::platform::sprawl_data_dir()?;
    let ledger_path = data_dir.join("ledger.sqlite");

    let (record_found, _is_encrypted) = if let Ok(conn) = rusqlite::Connection::open(&ledger_path) {
        let result = conn.query_row(
            "SELECT classification FROM secrets WHERE id = ?1",
            [&args.key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(_classification) => (true, true), // ledger entries are always encrypted
            Err(rusqlite::Error::QueryReturnedNoRows) => (false, false),
            Err(_) => (false, false),
        }
    } else {
        (false, false)
    };

    if !record_found {
        if is_json {
            println!("{}", serde_json::json!({"status": "error", "message": "Secret not found in ledger"}));
        } else {
            println!("Error: Secret '{}' not found in ledger.", args.key);
        }
        std::process::exit(6);
    }

    // has_keyring: check OS keyring
    let has_keyring = {
        let store = sprawl_sentinel::scanner::OsKeyringStore::new("sprawl-secret-store");
        store.has_secret(&args.key)
    };

    if is_json {
        println!(
            "{}",
            serde_json::json!({
                "status": "ok",
                "key": args.key,
                "ledger_found": true,
                "encrypted": true,
                "keyring_present": has_keyring,
                "blast_radius": "Unknown (archivist indexing not yet active)"
            })
        );
    } else {
        println!("Simulating revocation for key {}...\n", args.key);
        println!("  Ledger:   FOUND (encrypted: yes)");
        let kr_status = if has_keyring { "Entry present" } else { "Not found" };
        println!("  Keyring:  {}", kr_status);
        println!("  Graph:    Not available (requires real-archivist backend)\n");
        println!("Blast radius: Unknown (archivist indexing not yet active).");
    }

    Ok(())
}
