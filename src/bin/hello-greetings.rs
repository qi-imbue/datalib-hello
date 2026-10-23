//! The Hello package's applet: a "Greetings" card listing what one Hello
//! source has written, newest first. Each Hello source gets its own, and
//! its params name the source (`source`). The contract is datalib's
//! docs/dev/applets.md: write the component, bind a port, announce it,
//! and answer only requests that carry the gateway's secret.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, process, thread};

use serde_json::json;
use sha2::{Digest, Sha256};
use tiny_http::{Header, Response, Server};

const COMPONENT_JS: &str = include_str!("../greetings.js");
const COMPONENT_NAME: &str = "greetings";

fn main() {
    if let Err(e) = run() {
        let _ = writeln!(std::io::stderr(), "hello-greetings: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut port = 0u16;
    let mut frontend_dir = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-p" => {
                port = args
                    .next()
                    .and_then(|p| p.parse().ok())
                    .ok_or("-p takes a port")?
            }
            "--frontend-dir" => frontend_dir = args.next().map(PathBuf::from),
            _ => {}
        }
    }
    let frontend_dir = frontend_dir.ok_or("--frontend-dir is required")?;
    let secret =
        env::var("DATALIB_APPLET_SECRET").map_err(|_| "DATALIB_APPLET_SECRET is not set")?;
    let root = PathBuf::from(env::var("DATALIB_DAG_DATA_ROOT").unwrap_or_else(|_| ".".into()));
    let source = datalib_sdk::step::params()?
        .get("source")
        .and_then(|s| s.as_str())
        .ok_or("params.source names no source")?
        .to_string();
    exit_with_parent();

    write_frontend(&frontend_dir, &source)?;
    let server = Server::http(("127.0.0.1", port)).map_err(|e| format!("bind: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .ok_or("bound to something that is not an IP address")?
        .port();
    let host = format!("127.0.0.1:{port}");
    println!("DATALIB_APPLET_PORT={port}");
    let _ = std::io::stdout().flush();

    for request in server.incoming_requests() {
        let header = |name: &'static str| {
            request
                .headers()
                .iter()
                .find(|h| h.field.equiv(name))
                .map(|h| h.value.as_str().to_string())
        };
        // Loopback is not private: refuse anything the gateway did not send.
        let admitted = header("Host").as_deref() == Some(host.as_str())
            && header("X-Datalib-Applet-Secret").is_some_and(|s| same(&s, &secret));
        let response = if !admitted {
            Response::from_string("forbidden").with_status_code(403)
        } else if request.url().split('?').next() == Some("/greetings") {
            Response::from_string(greetings(&root, &source).to_string()).with_header(
                Header::from_bytes("Content-Type", "application/json").expect("a valid header"),
            )
        } else {
            Response::from_string("not found").with_status_code(404)
        };
        let _ = request.respond(response);
    }
    Ok(())
}

fn write_frontend(dir: &Path, source: &str) -> Result<(), String> {
    let namespace = dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("--frontend-dir must end in the namespace")?;
    fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let hash = hex(&Sha256::digest(COMPONENT_JS.as_bytes()));
    let js = dir.join(format!("{hash}.js"));
    fs::write(&js, COMPONENT_JS).map_err(|e| format!("write {}: {e}", js.display()))?;
    let meta = json!({
        "title": format!("Greetings · {source}"),
        "description": format!("What the Hello source {source} has written."),
        "component_hash": hash,
        // The namespace is the applet's id; the card calls /applet/<id>/.
        "component_args": [namespace],
    });
    let meta_path = dir.join(format!("{COMPONENT_NAME}.json"));
    fs::write(&meta_path, meta.to_string())
        .map_err(|e| format!("write {}: {e}", meta_path.display()))
}

/// What the source's ingest wrote to `<root>/<source>/ingest/greetings.jsonl`,
/// newest first. A line that does not parse is left out.
fn greetings(root: &Path, source: &str) -> serde_json::Value {
    let file = root.join(source).join("ingest").join("greetings.jsonl");
    let text = fs::read_to_string(file).unwrap_or_default();
    let mut rows: Vec<serde_json::Value> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    rows.reverse();
    json!({ "source": source, "greetings": rows })
}

/// The gateway holds stdin open for as long as it runs; EOF means it is
/// gone, and an applet that outlives it holds its port forever.
fn exit_with_parent() {
    if env::var("DATALIB_PARENT_PIPE").as_deref() != Ok("0") {
        return;
    }
    thread::spawn(|| {
        let mut buf = [0u8; 4096];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut buf), Ok(n) if n > 0) {}
        process::exit(0);
    });
}

fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
