//! The Hello package's applet: a "Greetings" card listing what each Hello
//! source in the library last wrote. The contract is datalib's
//! docs/dev/applets.md: write the component, bind a port, announce it,
//! and answer only requests that carry the gateway's secret.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
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
    exit_with_parent();

    write_frontend(&frontend_dir)?;
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
            Response::from_string(greetings(&root).to_string()).with_header(
                Header::from_bytes("Content-Type", "application/json").expect("a valid header"),
            )
        } else {
            Response::from_string("not found").with_status_code(404)
        };
        let _ = request.respond(response);
    }
    Ok(())
}

fn write_frontend(dir: &Path) -> Result<(), String> {
    let namespace = dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("--frontend-dir must end in the namespace")?;
    fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let hash = hex(&Sha256::digest(COMPONENT_JS.as_bytes()));
    let js = dir.join(format!("{hash}.js"));
    fs::write(&js, COMPONENT_JS).map_err(|e| format!("write {}: {e}", js.display()))?;
    let meta = json!({
        "title": "Greetings",
        "description": "What each Hello source in this library last wrote.",
        "component_hash": hash,
        // The namespace is the applet's id; the card calls /applet/<id>/.
        "component_args": [namespace],
    });
    let meta_path = dir.join(format!("{COMPONENT_NAME}.json"));
    fs::write(&meta_path, meta.to_string())
        .map_err(|e| format!("write {}: {e}", meta_path.display()))
}

/// A Hello source's ingest writes `<root>/<source id>/ingest/hello.txt`:
/// a first line, then the params it ran with, as JSON.
fn greetings(root: &Path) -> serde_json::Value {
    let mut out = Vec::new();
    let mut sources: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    sources.sort();
    for source in sources {
        let file = source.join("ingest").join("hello.txt");
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        let params: serde_json::Value = text
            .split_once('\n')
            .and_then(|(_, rest)| serde_json::from_str(rest).ok())
            .unwrap_or_default();
        let written_ms = fs::metadata(&file)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);
        out.push(json!({
            "source": source.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            "greeting": params.get("greeting").and_then(|g| g.as_str()).unwrap_or(""),
            "written_ms": written_ms,
        }));
    }
    serde_json::Value::Array(out)
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
