//! What a step reads from the runner and says back to it: datalib's
//! docs/dev/step_protocol.md.

use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{Error, Result};

/// The step's own environment.
pub struct Step {
    /// `<group>/<function>`: the tree it writes, under the data root.
    pub id: String,
    pub group: String,
    pub data_root: PathBuf,
    /// The steps it reads, by id.
    pub inputs: Vec<String>,
    /// Asked to empty its store (`--reset store`) rather than run.
    pub reset: bool,
}

impl Step {
    pub fn from_env() -> Result<Step> {
        let var = |k: &str| std::env::var(k).map_err(|_| Error(format!("{k} is not set")));
        let id = var("DATALIB_DAG_STEP")?;
        let group = var("DATALIB_DAG_GROUP")?;
        let data_root = std::env::var("DATALIB_DAG_DATA_ROOT")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .map_err(|e| Error(format!("the data root: {e}")))?;
        let inputs = std::env::var("DATALIB_DAG_INPUTS")
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
        let args: Vec<String> = std::env::args().collect();
        let reset = args
            .windows(2)
            .any(|w| w[0] == "--reset" && w[1] == "store");
        Ok(Step {
            id,
            group,
            data_root,
            inputs,
            reset,
        })
    }

    /// The tree this step writes.
    pub fn tree(&self) -> PathBuf {
        self.data_root.join(&self.id)
    }
}

/// The step's params, from `--params-file`; an empty object without one.
/// Read it early: the runner deletes the file when the step exits.
pub fn params() -> std::result::Result<serde_json::Map<String, Value>, String> {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args
        .windows(2)
        .find(|w| w[0] == "--params-file")
        .map(|w| w[1].clone())
    else {
        return Ok(Default::default());
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    match serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))? {
        Value::Object(m) => Ok(m),
        _ => Err(format!("{path}: params are not an object")),
    }
}

/// Now, as an RFC 3339 string with its offset: the run's pinned now
/// (`DATALIB_DAG_NOW`) when the runner gives one.
pub fn now() -> String {
    std::env::var("DATALIB_DAG_NOW").unwrap_or_else(|_| {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("a current time formats")
    })
}

/// `now()` split the way datalib stores a stamp: the instant in UTC, and
/// the offset it was read in.
pub fn now_utc_and_offset() -> (String, String) {
    let at = parse_time(&now()).unwrap_or_else(OffsetDateTime::now_utc);
    let utc = at
        .to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("a time formats");
    let (h, m, _) = at.offset().as_hms();
    let sign = if at.offset().is_negative() { '-' } else { '+' };
    (utc, format!("{sign}{:02}:{:02}", h.abs(), m.abs()))
}

pub fn parse_time(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// Unix milliseconds of an RFC 3339 stamp.
pub fn unix_ms(s: &str) -> Option<i64> {
    parse_time(s).map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
}

fn emit(event: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{event}");
    let _ = out.flush();
}

/// A counter the Manage screen shows: `items`, `documents`, or
/// `problems` with a `severity` label.
pub fn metric(step: &str, name: &str, labels: Option<Value>, value: i64) {
    let mut e = json!({ "event": "metric", "step": step, "name": name, "value": value });
    if let Some(l) = labels {
        e["labels"] = l;
    }
    emit(e);
}

/// That nothing went wrong: zero errors and zero warnings, said every run
/// so the Manage screen's counts clear.
pub fn no_problems(step: &str) {
    for severity in ["error", "warning"] {
        metric(step, "problems", Some(json!({ "severity": severity })), 0);
    }
}

/// The step's last line: the tree it wrote, at `version`.
pub fn outcome(step: &str, version: &str, rows: Option<i64>) {
    let mut output = json!({ "path": step, "version": version });
    if let Some(r) = rows {
        output["rows"] = json!(r);
    }
    emit(json!({ "event": "outcome", "outputs": [output] }));
}

/// The step failed, and why: `data`, `transient`, …
pub fn failure(kind: &str) {
    emit(json!({ "event": "outcome", "failure": kind }));
}
