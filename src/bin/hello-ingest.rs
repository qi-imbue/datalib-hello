//! The Hello source's one step: writes a greeting and the params it was
//! given into the tree it owns, `$DATALIB_DAG_STEP` under the data root.
//! The contract is datalib's docs/dev/step_protocol.md.

use std::path::PathBuf;
use std::{env, fs, process};

fn main() {
    if let Err(e) = run() {
        eprintln!("hello-ingest: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let step = env::var("DATALIB_DAG_STEP").map_err(|_| "DATALIB_DAG_STEP is not set")?;
    let mut params_file = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--params-file" {
            params_file = args.next();
        }
    }
    let dir = PathBuf::from(step);
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let mut out = String::from("hello from a package\n");
    if let Some(path) = params_file {
        out += &fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    }
    let file = dir.join("hello.txt");
    fs::write(&file, out).map_err(|e| format!("write {}: {e}", file.display()))
}
