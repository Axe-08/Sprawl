use clap::Args;
use sprawl_archaeologist::bundle::{BundleOptions, Bundler};
use sprawl_core::Result;
use std::path::PathBuf;

#[derive(Args)]
pub struct BundleArgs {
    /// The directory to bundle
    pub dir: PathBuf,
    /// Maximum tokens allowed
    #[arg(long, default_value_t = 32768)]
    pub max_tokens: usize,
    /// Output file path (if omitted, prints to stdout)
    #[arg(long)]
    pub output: Option<PathBuf>,
}

pub fn handle(args: &BundleArgs, is_json: bool) -> Result<()> {
    let bundler = Bundler::new();
    let opts = BundleOptions {
        max_tokens: args.max_tokens,
        output_path: args.output.clone(),
    };

    match bundler.bundle_directory(&args.dir, &opts) {
        Ok(content) => {
            if let Some(out_path) = &args.output {
                if let Err(e) = std::fs::write(out_path, content) {
                    return Err(sprawl_core::SprawlError::Other(format!(
                        "Failed to write bundle: {}",
                        e
                    )));
                } else {
                    if is_json {
                        println!(
                            "{}",
                            serde_json::json!({"status": "ok", "path": out_path.display().to_string()})
                        );
                    } else {
                        println!("Bundle written to {}", out_path.display());
                    }
                }
            } else {
                if is_json {
                    println!(
                        "{}",
                        serde_json::json!({"status": "ok", "content": content})
                    );
                } else {
                    println!("{}", content);
                }
            }
            Ok(())
        }
        Err(e) => Err(sprawl_core::SprawlError::Other(format!(
            "Failed to bundle: {}",
            e
        ))),
    }
}
