pub fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    
    // Check ignored directories
    if entry.file_type().is_dir() {
        return matches!(
            name.as_ref(),
            ".git" | "target" | "node_modules" | "dist" | "build" | ".venv" | "venv" | "env" | "site-packages" | "__pycache__" | ".next" | ".idea" | ".vscode" | "vendor" | ".dart_tool" | "cache" | "bin" | "obj" | "out" | "backup"
        );
    }
    
    // Check ignored files
    if entry.file_type().is_file() {
        let name_str = name.as_ref();
        
        // Ignore lockfiles, minified files, source maps, databases, and heavy cache files
        if name_str.ends_with(".lock") 
            || name_str == "package-lock.json" 
            || name_str == "yarn.lock"
            || name_str.ends_with(".map") 
            || name_str.ends_with(".min.js") 
            || name_str.ends_with(".min.css")
            || name_str.ends_with(".sqlite")
            || name_str.ends_with(".db")
            || name_str.ends_with(".filecache")
            || name_str == "licenses_flutter"
            || name_str == "licenses_fuchsia"
            || name_str == "RECORD"
            || name_str == "cacert.pem"
            || name_str == "roots.pem"
        {
            return true;
        }

        // Ignore common media, binary, and heavy data formats
        let ext = std::path::Path::new(name_str).extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp" | "eps" |
            "mp4" | "webm" | "wav" | "mp3" | "pdf" | "zip" | "tar" | "gz" | 
            "bin" | "wasm" | "so" | "dll" | "dylib" | "exe" | "class" | "jar" | 
            "pack" | "idx" | "csv" | "tsv" | "symbols" | "unity" | "prefab" | "mat" | "asset"
        ) {
            return true;
        }

        // Ignore large files (e.g., > 1MB)
        if let Ok(metadata) = entry.metadata() {
            if metadata.len() > 1_048_576 { // 1 MB
                return true;
            }
        }
    }
    
    false
}
