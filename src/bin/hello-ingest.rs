//! The Hello source's ingest: each run appends the greeting it was given,
//! and when, to `greetings.jsonl` in the tree it owns, `$DATALIB_DAG_STEP`
//! under the data root. The render steps read that file. The contract is
//! datalib's docs/dev/step_protocol.md.

use std::io::Write;
use std::path::PathBuf;
use std::{env, fs, process};

use serde_json::json;

fn main() {
    if let Err(e) = run() {
        eprintln!("hello-ingest: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let step = env::var("DATALIB_DAG_STEP").map_err(|_| "DATALIB_DAG_STEP is not set")?;
    let params = datalib_sdk::step::params()?;
    let greeting = params
        .get("greeting")
        .and_then(|g| g.as_str())
        .unwrap_or("hello");
    let dir = PathBuf::from(step);
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let file = dir.join("greetings.jsonl");
    let line = json!({ "at": datalib_sdk::step::now(), "greeting": greeting });
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .and_then(|mut f| writeln!(f, "{line}"))
        .map_err(|e| format!("write {}: {e}", file.display()))
}
