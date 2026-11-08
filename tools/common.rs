//! What read_cut_files and export_models share: where the repositories are,
//! Python's os.path.relpath, json.dump(indent=1), and running a sibling
//! repository's built Rust for its JSON.
use lasermade_tools::turn3d::J;
use std::path::{Component, Path, PathBuf};

/// bore-tmm's own folder
pub fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// ~/LaserMadeMusic/GIT, the folder that holds the repositories side by side
pub fn git() -> PathBuf {
    normpath(&here().join(".."))
}

/// os.path.normpath of an absolute path
pub fn normpath(p: &Path) -> PathBuf {
    let mut out = PathBuf::from("/");
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
            _ => {}
        }
    }
    out
}

/// os.path.abspath
pub fn abspath(s: &str) -> PathBuf {
    let p = Path::new(s);
    if p.is_absolute() {
        normpath(p)
    } else {
        normpath(&std::env::current_dir().unwrap().join(p))
    }
}

/// os.path.relpath(path, start), both absolute
pub fn relpath(path: &Path, start: &Path) -> String {
    let a: Vec<_> = normpath(path).components().skip(1).map(|c| c.as_os_str().to_owned()).collect();
    let b: Vec<_> = normpath(start).components().skip(1).map(|c| c.as_os_str().to_owned()).collect();
    let n = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".into(); b.len() - n];
    parts.extend(a[n..].iter().map(|s| s.to_string_lossy().into_owned()));
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

/// json.dump(d, f, indent=1) and a newline
pub fn write_json(path: &Path, d: &J) {
    let mut s = String::new();
    d.dump_indent(1, 0, &mut s);
    s.push('\n');
    std::fs::write(path, s).unwrap_or_else(|e| lasermade_tools::die(format!("{}: {e}", path.display())));
}

/// A sibling repository's built program, run for the JSON it prints: Err with
/// what went wrong, in read_cut_files' words.
pub fn rust(repo: &str, exe: &str, args: &[&str]) -> Result<J, String> {
    let path = git().join(repo).join("target").join("release").join(exe);
    let shown = relpath(&path, &git());
    if !path.exists() {
        return Err(format!("{shown} is not built: cargo build --release in {repo}"));
    }
    let r = std::process::Command::new(&path).args(args).output().map_err(|e| format!("{shown}: {e}"))?;
    let out = String::from_utf8_lossy(&r.stdout).into_owned();
    if !r.status.success() {
        let err = String::from_utf8_lossy(&r.stderr);
        return Err(format!("{shown} {}: {}", args.join(" "), format!("{out}{err}").trim()));
    }
    J::parse(&out).map_err(|e| format!("{shown}: {e}"))
}

/// d.items() of a JSON object
pub fn items(d: &J) -> Vec<(String, J)> {
    match d {
        J::O(kv) => kv.clone(),
        _ => vec![],
    }
}
