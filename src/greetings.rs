//! What the Hello ingest wrote: one greeting per sync, in
//! `<group>/ingest/greetings.jsonl`. Shared by the two render steps.

use std::path::Path;

pub struct Greeting {
    /// RFC 3339, with the offset of the clock it was read on.
    pub at: String,
    pub greeting: String,
}

/// Every greeting the ingest tree holds, oldest first. A line that does
/// not parse is left out and counted.
pub fn read(ingest_tree: &Path) -> (Vec<Greeting>, usize) {
    let text = std::fs::read_to_string(ingest_tree.join("greetings.jsonl")).unwrap_or_default();
    let mut out = Vec::new();
    let mut bad = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                bad += 1;
                continue;
            }
        };
        match (v["at"].as_str(), v["greeting"].as_str()) {
            (Some(at), Some(g)) if datalib_sdk::step::unix_ms(at).is_some() => out.push(Greeting {
                at: at.to_string(),
                greeting: g.to_string(),
            }),
            _ => bad += 1,
        }
    }
    (out, bad)
}

/// Run `body`, and on an error say so the way the step protocol asks.
pub fn run_step(name: &str, body: impl FnOnce() -> Result<(), String>) {
    if let Err(e) = body() {
        eprintln!("{name}: {e}");
        datalib_sdk::step::failure("data");
        std::process::exit(1);
    }
}
