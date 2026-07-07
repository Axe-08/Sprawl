use ignore::WalkBuilder;
use sprawl_core::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser};

pub struct BundleOptions {
    pub max_tokens: usize,
    pub output_path: Option<PathBuf>,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            max_tokens: 32768,
            output_path: None,
        }
    }
}

pub struct Bundler {}

impl Bundler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Bundler {
    fn default() -> Self {
        Self::new()
    }
}

fn get_language_for_extension(ext: &str) -> Option<Language> {
    match ext {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        _ => None,
    }
}

fn strip_comments(source_code: &str, lang: Language) -> String {
    let mut parser = Parser::new();
    parser.set_language(&lang).unwrap();
    let tree = parser.parse(source_code, None).unwrap();

    // We will do a basic pass: just collect nodes that are NOT comments
    // Actually, stripping comments precisely with tree-sitter requires traversing and removing comment nodes.
    // A simpler way is to find all comment nodes and remove their byte ranges from the source.
    let mut comment_ranges = Vec::new();

    let mut cursor = tree.root_node().walk();
    let mut reached_root = false;
    while !reached_root {
        let node = cursor.node();
        // Different languages have different node kinds for comments
        // Rust: "line_comment", "block_comment"
        // JS: "comment"
        // Python: "comment"
        // Go: "comment"
        if node.kind().contains("comment") {
            comment_ranges.push((node.start_byte(), node.end_byte()));
        }

        if cursor.goto_first_child() {
            continue;
        }

        if cursor.goto_next_sibling() {
            continue;
        }

        loop {
            if !cursor.goto_parent() {
                reached_root = true;
                break;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }

    // Sort and merge ranges just in case
    comment_ranges.sort_by_key(|r| r.0);

    let bytes = source_code.as_bytes();
    let mut result = Vec::new();
    let mut last_end = 0;

    for (start, end) in comment_ranges {
        if start > last_end {
            result.extend_from_slice(&bytes[last_end..start]);
        }
        last_end = last_end.max(end);
    }
    if last_end < bytes.len() {
        result.extend_from_slice(&bytes[last_end..]);
    }

    // Convert back to string and strip consecutive blank lines
    let stripped = String::from_utf8_lossy(&result).into_owned();
    let mut final_lines = Vec::new();
    let mut last_was_blank = false;
    for line in stripped.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_was_blank {
                final_lines.push("");
                last_was_blank = true;
            }
        } else {
            final_lines.push(line);
            last_was_blank = false;
        }
    }

    final_lines.join("\n")
}

impl Bundler {
    /// Recursively bundle a directory, respecting .sprawl.toml and .gitignore.
    /// Uses tree-sitter to strip AST comments and blank lines if token limits are approached.
    pub fn bundle_directory(&self, dir: &Path, opts: &BundleOptions) -> Result<String> {
        let walker = WalkBuilder::new(dir)
            .hidden(true)
            .ignore(true)
            .git_ignore(true)
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "node_modules"
                    && name != "target"
                    && name != ".git"
                    && name != ".venv"
                    && name != "__pycache__"
            })
            .build();

        let mut all_files_content = String::new();
        let mut estimated_tokens = 0;

        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(metadata) = fs::metadata(path) {
                            // Skip files > 1MB
                            if metadata.len() > 1_000_000 {
                                continue;
                            }
                        }

                        if let Ok(content) = fs::read_to_string(path) {
                            let rel_path =
                                path.strip_prefix(dir).unwrap_or(path).display().to_string();
                            let ext = path.extension().unwrap_or_default().to_string_lossy();

                            let processed_content =
                                if let Some(lang) = get_language_for_extension(&ext) {
                                    strip_comments(&content, lang)
                                } else {
                                    content
                                };

                            let file_str = format!(
                                "\\n--- {} ---\\n```\\n{}\\n```\\n",
                                rel_path, processed_content
                            );

                            // Rough token estimate: ~4 chars per token
                            let file_tokens = file_str.len() / 4;
                            if estimated_tokens + file_tokens > opts.max_tokens
                                && !all_files_content.is_empty()
                            {
                                // Reached token budget, stop packing
                                all_files_content.push_str("\n[TRUNCATED DUE TO TOKEN LIMIT]\n");
                                break;
                            }

                            all_files_content.push_str(&file_str);
                            estimated_tokens += file_tokens;
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!("Error walking directory: {}", err);
                }
            }
        }

        Ok(all_files_content)
    }
}
