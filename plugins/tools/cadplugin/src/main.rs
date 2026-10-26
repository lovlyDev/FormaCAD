//! `cadplugin pack`: turn a `wasm32-unknown-unknown` build made with
//! wit-bindgen into a WebAssembly component and pack it with the manifest,
//! translations and assets into `<id>-<version>.cadplugin`.
//!
//! ```text
//! cadplugin pack --manifest manifest.json --wasm target/.../plugin.wasm [--dir .] [--out dist]
//! ```
//! `--dir` is the plugin project root: `locales/*.json`, `assets/**` and the
//! manifest's PNG icon are taken from there when present.
use std::{
    io::Write,
    path::{Path, PathBuf},
};

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("cadplugin: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut args = args.into_iter();
    if args.next().as_deref() != Some("pack") {
        return Err("usage: cadplugin pack --manifest <file> --wasm <file> [--dir <project>] [--out <dir>]".into());
    }
    let (mut manifest, mut wasm, mut dir, mut out) = (None, None, PathBuf::from("."), PathBuf::from("dist"));
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--wasm" => wasm = Some(PathBuf::from(value)),
            "--dir" => dir = PathBuf::from(value),
            "--out" => out = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    let manifest_path = manifest.ok_or("--manifest is required")?;
    let manifest_bytes = std::fs::read(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| format!("manifest.json: {e}"))?;
    let field = |name: &str| manifest[name].as_str().map(str::to_owned).ok_or(format!("manifest.{name} is missing"));
    let (id, version, entry) = (field("id")?, field("version")?, field("entry")?);
    let wasm_path = wasm.ok_or("--wasm is required")?;
    let module = std::fs::read(&wasm_path).map_err(|e| format!("{}: {e}", wasm_path.display()))?;
    let component = componentize(&module)?;

    let mut files: Vec<(String, Vec<u8>)> = vec![("manifest.json".into(), manifest_bytes), (entry, component)];
    if let Some(icon) = manifest["icon"].as_str().filter(|i| !i.starts_with("lucide:")) {
        files.push((icon.to_owned(), std::fs::read(dir.join(icon)).map_err(|e| format!("{icon}: {e}"))?));
    }
    for folder in ["locales", "assets"] {
        collect(&dir.join(folder), folder, &mut files)?;
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let target = out.join(format!("{id}-{version}.cadplugin"));
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        // Fixed timestamp: the same input gives the same archive and hash.
        .last_modified_time(zip::DateTime::default());
    files.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, data) in &files {
        writer.start_file(name.as_str(), options).map_err(|e| e.to_string())?;
        writer.write_all(data).map_err(|e| e.to_string())?;
    }
    let bytes = writer.finish().map_err(|e| e.to_string())?.into_inner();
    std::fs::write(&target, &bytes).map_err(|e| e.to_string())?;
    use sha2::Digest;
    println!("{} ({} bytes, sha256 {:x})", target.display(), bytes.len(), sha2::Sha256::digest(&bytes));
    Ok(())
}

/// Already a component (layer 1) → keep; core module with wit-bindgen
/// metadata → encode as a component. Unused world imports are dropped by the
/// encoder, so the component imports only what the plugin really calls.
fn componentize(module: &[u8]) -> Result<Vec<u8>, String> {
    if module.len() >= 8 && module[4..8] != [1, 0, 0, 0] {
        return Ok(module.to_vec());
    }
    wit_component::ComponentEncoder::default()
        .module(module)
        .and_then(|encoder| encoder.validate(true).encode())
        .map_err(|e| format!("componentize: {e:#}"))
}

fn collect(path: &Path, prefix: &str, files: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(path) else { return Ok(()) };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!("{name}: symbolic links are not allowed"));
        } else if kind.is_dir() {
            collect(&entry.path(), &name, files)?;
        } else {
            files.push((name, std::fs::read(entry.path()).map_err(|e| e.to_string())?));
        }
    }
    Ok(())
}
