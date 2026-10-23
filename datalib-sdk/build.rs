// Compiles the doltlite amalgamation that datalib pins, with the defines
// datalib builds it with (imbue-ai/datalib third-party/doltlite/BUILD.bazel),
// into this crate. The zip is fetched once into OUT_DIR and checked against
// the pinned sha256; set DOLTLITE_AMALGAMATION_DIR to a directory holding
// doltlite.c to build offline.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const VERSION: &str = "0.50.16";
const SHA256: &str = "bf8887cdfbce09a4f6d42b362ad1848d6fa4b9cb49437afbbd73d9ff2883f729";

fn main() {
    println!("cargo:rerun-if-env-changed=DOLTLITE_AMALGAMATION_DIR");
    let dir = match std::env::var_os("DOLTLITE_AMALGAMATION_DIR") {
        Some(d) => PathBuf::from(d),
        None => fetch(&PathBuf::from(std::env::var_os("OUT_DIR").unwrap())),
    };
    let mut build = cc::Build::new();
    build
        .file(dir.join("doltlite.c"))
        .define("DOLTLITE_PROLLY", "1")
        .define("DOLTLITE_VERSION", format!("\"{VERSION}\"").as_str())
        .define("SQLITE_CORE", None)
        .define("SQLITE_THREADSAFE", "1")
        .define("SQLITE_ENABLE_JSON1", None)
        .define("SQLITE_ENABLE_COLUMN_METADATA", None)
        .opt_level(2)
        .warnings(false)
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-deprecated-non-prototype");
    build.compile("doltlite");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }
}

fn fetch(out: &Path) -> PathBuf {
    let dir = out.join(format!("doltlite-amalgamation-{VERSION}"));
    if dir.join("doltlite.c").exists() {
        return dir;
    }
    let zip = out.join("doltlite.zip");
    let url = format!(
        "https://github.com/dolthub/doltlite/releases/download/v{VERSION}/doltlite-amalgamation-{VERSION}.zip"
    );
    run(Command::new("curl")
        .args(["-sSfL", "-o"])
        .arg(&zip)
        .arg(&url));
    let got = format!("{:x}", Sha256::digest(std::fs::read(&zip).unwrap()));
    assert_eq!(got, SHA256, "{url} does not hash to the pinned sha256");
    run(Command::new("unzip")
        .args(["-q", "-o"])
        .arg(&zip)
        .arg("-d")
        .arg(out));
    dir
}

fn run(cmd: &mut Command) {
    let status = cmd.status().unwrap_or_else(|e| panic!("{cmd:?}: {e}"));
    assert!(status.success(), "{cmd:?} failed: {status}");
}
