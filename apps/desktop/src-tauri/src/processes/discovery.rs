use std::path::{Path, PathBuf};

pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut directories: Vec<PathBuf> = std::env::split_paths(&path).collect();
    // Finder and Linux desktop launchers do not inherit an interactive shell PATH.
    if cfg!(unix) {
        directories
            .extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"].map(PathBuf::from));
        if let Some(base) = directories::BaseDirs::new() {
            directories.push(base.home_dir().join(".local/bin"));
            directories.push(base.home_dir().join(".cargo/bin"));
        }
    }
    for dir in directories {
        if !dir.is_absolute() {
            continue;
        }
        let p = dir.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        if p.is_file() {
            return p.canonicalize().ok();
        }
    }
    None
}
pub fn find_codex() -> Option<PathBuf> {
    if let Some(p) = find_executable("codex") {
        return Some(p);
    }
    let base = directories::BaseDirs::new()?;
    let roots = [
        base.config_dir()
            .join("npm/node_modules/@openai/codex/node_modules/@openai"),
        base.home_dir()
            .join(".local/share/pnpm/global/5/node_modules/@openai/codex/node_modules/@openai"),
    ];
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let vendor = entry.path().join("vendor");
                if let Ok(targets) = std::fs::read_dir(vendor) {
                    for target in targets.flatten() {
                        for folder in ["bin", "codex"] {
                            let p = target.path().join(folder).join(if cfg!(windows) {
                                "codex.exe"
                            } else {
                                "codex"
                            });
                            if p.is_file() {
                                return p.canonicalize().ok();
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
pub fn python(root: &Path) -> Option<PathBuf> {
    let configured = root
        .parent()
        .and_then(|parent| std::fs::read_to_string(parent.join("cad-python.txt")).ok())
        .map(|value| PathBuf::from(value.trim()));
    configured
        .filter(|p| p.is_absolute() && p.is_file())
        .or_else(|| {
            std::env::var_os("FORMA_PYTHON")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute() && p.is_file())
        })
        .or_else(|| {
            let p = root.parent()?.join(if cfg!(windows) {
                "cad-env/Scripts/python.exe"
            } else {
                "cad-env/bin/python"
            });
            p.is_file().then_some(p)
        })
        .or_else(|| find_executable("python3"))
        .or_else(|| find_executable("python"))
}
