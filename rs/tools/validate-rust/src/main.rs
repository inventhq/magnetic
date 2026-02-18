use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::{fs, path::Path};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "magnetic-validate", about = "Validate manifests JSON")]
struct Args {
    /// Path to manifests directory (e.g., manifests/)
    dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = Path::new(&args.dir);
    if !root.exists() {
        return Err(anyhow!("path not found: {}", root.display()));
    }

    // 1) Validate all *.json parse
    let mut count = 0usize;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let path = entry.path();
        let data = fs::read_to_string(path)
            .with_context(|| format!("read failed: {}", path.display()))?;
        // strict JSON parse (no comments, trailing commas)
        let v: serde_json::Value =
            serde_json::from_str(&data).with_context(|| format!("json parse failed: {}", path.display()))?;
        // basic shape checks for .well-known
        if path.components().any(|c| c.as_os_str() == ".well-known") {
            validate_well_known_file(path, &v)?;
        }
        count += 1;
    }

    if count == 0 {
        return Err(anyhow!("no JSON files found under {}", root.display()));
    }

    Ok(())
}

fn validate_well_known_file(path: &std::path::Path, v: &serde_json::Value) -> Result<()> {
    let file = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let obj = v.as_object().ok_or_else(|| anyhow!("{}: top-level must be an object", file))?;

    // common: version must be integer >= 1
    let version = obj
        .get("version")
        .ok_or_else(|| anyhow!("{}: missing 'version'", file))?
        .as_i64()
        .ok_or_else(|| anyhow!("{}: 'version' must be integer", file))?;
    if version < 1 {
        return Err(anyhow!("{}: 'version' must be >= 1", file));
    }

    // file-specific minimal shape checks
    match file {
        "capabilities.json" => {
            must_be_array(obj, "capabilities", file)?;
        }
        "streams.json" => {
            must_be_array(obj, "streams", file)?;
        }
        "actions.json" => {
            must_be_array(obj, "actions", file)?;
        }
        "errors.json" => {
            must_be_object(obj, "errors", file)?;
        }
        "simulators.json" => {
            // allow empty object for now
            must_be_object(obj, "simulators", file)?;
        }
        _ => {}
    }
    Ok(())
}

fn must_be_array(obj: &serde_json::Map<String, serde_json::Value>, key: &str, file: &str) -> Result<()> {
    obj.get(key)
        .ok_or_else(|| anyhow!("{}: missing '{}'", file, key))?
        .as_array()
        .ok_or_else(|| anyhow!("{}: '{}' must be array", file, key))?;
    Ok(())
}

fn must_be_object(obj: &serde_json::Map<String, serde_json::Value>, key: &str, file: &str) -> Result<()> {
    obj.get(key)
        .ok_or_else(|| anyhow!("{}: missing '{}'", file, key))?
        .as_object()
        .ok_or_else(|| anyhow!("{}: '{}' must be object", file, key))?;
    Ok(())
}
