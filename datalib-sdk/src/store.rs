//! A doltlite store held by its one writer, the way datalib holds one:
//! a lock file beside it, every write on the `datalib_writer` branch, and
//! `main` moved to it only when a run seals, so a reader (which follows
//! `main`) never sees half a run. Copied from datalib's
//! `etl/src/doltlite_raw.rs` (MIT, Copyright imbue-ai).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::sqlite::{Connection, Value};
use crate::{Error, Result};

const WRITER_BRANCH: &str = "datalib_writer";

/// `_datalib_meta`, byte for byte: its text is part of every schema hash.
pub const META_DDL: &str = "CREATE TABLE IF NOT EXISTS _datalib_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    written_at_utc TEXT NOT NULL,
    tz_offset TEXT NULL
)";

/// What `_datalib_meta.datalib_version` says. datalib refuses a whole data
/// root when a store names a newer datalib than the one running, and
/// never counts a value it cannot parse as one: so this names the SDK.
const WRITER: &str = concat!("datalib-sdk/", env!("CARGO_PKG_VERSION"));

/// A store's shape, as datalib records it and compares it: blake3 over
/// `_datalib_meta`'s DDL and then the store's, each followed by a newline.
pub fn schema_hash(ddl: &[&str]) -> String {
    let mut h = blake3::Hasher::new();
    for stmt in std::iter::once(&META_DDL).chain(ddl) {
        h.update(stmt.as_bytes());
        h.update(b"\n");
    }
    h.finalize().to_hex().to_string()
}

/// Which of datalib's stores a file is.
#[derive(Debug, Clone, Copy)]
pub enum StoreKind {
    Render,
    TimeSeries,
}

impl StoreKind {
    fn as_str(self) -> &'static str {
        match self {
            StoreKind::Render => "render",
            StoreKind::TimeSeries => "time_series",
        }
    }
}

pub struct Store {
    conn: Connection,
    _lock: File,
    path: PathBuf,
}

impl Store {
    /// Open `path` as its one writer, creating it with `ddl` (which the
    /// schema hash covers) and `indexes` (which it does not). A store
    /// written in another shape is emptied: what a render step writes can
    /// always be written again.
    pub fn open(path: &Path, kind: StoreKind, ddl: &[&str], indexes: &[&str]) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error(format!("{}: {e}", dir.display())))?;
        }
        let lock = take_lock(path)?;
        let conn = Connection::open(path)?;
        let store = Store {
            conn,
            _lock: lock,
            path: path.to_path_buf(),
        };
        store.on_writer_branch()?;
        store.clean_up_a_crashed_run()?;
        store.publish()?;
        let hash = schema_hash(ddl);
        let current = store
            .conn
            .query(
                "SELECT value FROM _datalib_meta WHERE key = 'schema_hash'",
                &[],
            )
            .ok()
            .and_then(|rows| rows.into_iter().next())
            .and_then(|r| r.into_iter().next());
        if current.as_ref().and_then(Value::as_str) != Some(hash.as_str()) {
            for table in store.tables()? {
                if table != "_datalib_meta" {
                    store
                        .conn
                        .execute(&format!("DROP TABLE IF EXISTS \"{table}\""), &[])?;
                }
            }
        }
        store.conn.execute(META_DDL, &[])?;
        for stmt in ddl.iter().chain(indexes) {
            store.conn.execute(stmt, &[])?;
        }
        store.write_meta(kind, &hash)?;
        store.seal("schema: apply DDL")?;
        Ok(store)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn on_writer_branch(&self) -> Result<()> {
        if self
            .conn
            .execute("SELECT dolt_connect_branch(?)", &[WRITER_BRANCH.into()])
            .is_err()
        {
            self.conn
                .execute("SELECT dolt_checkout('-b', ?)", &[WRITER_BRANCH.into()])?;
        }
        let branch = self.conn.one("SELECT active_branch()", &[])?;
        if branch.as_str() != Some(WRITER_BRANCH) {
            return Err(Error(format!(
                "{}: on branch {branch:?}, not {WRITER_BRANCH}",
                self.path.display()
            )));
        }
        Ok(())
    }

    /// What a run that died left uncommitted is thrown away.
    fn clean_up_a_crashed_run(&self) -> Result<()> {
        let dirty = !self
            .conn
            .query("SELECT * FROM dolt_status", &[])?
            .is_empty();
        if dirty {
            let commits = self
                .conn
                .query("SELECT commit_hash FROM dolt_log", &[])?
                .len();
            if commits > 1 {
                self.conn.execute("SELECT dolt_reset('--hard')", &[])?;
            }
            self.conn.execute("SELECT dolt_clean()", &[])?;
        }
        Ok(())
    }

    fn tables(&self) -> Result<Vec<String>> {
        Ok(self
            .conn
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                 AND name NOT LIKE 'dolt_%'",
                &[],
            )?
            .into_iter()
            .filter_map(|r| {
                r.into_iter()
                    .next()
                    .and_then(|v| v.as_str().map(String::from))
            })
            .collect())
    }

    fn write_meta(&self, kind: StoreKind, hash: &str) -> Result<()> {
        let doltlite = self.conn.one("SELECT dolt_version()", &[])?;
        let (now, offset) = crate::step::now_utc_and_offset();
        let rows: [(&str, Value); 7] = [
            ("datalib_version", WRITER.into()),
            ("git_hash", "unknown".into()),
            ("doltlite_version", doltlite),
            ("schema_hash", hash.into()),
            ("schema_version", "0".into()),
            ("shared_schema_version", "0".into()),
            ("store_kind", kind.as_str().into()),
        ];
        for (key, value) in rows {
            let same = self.conn.one(
                "SELECT value = ? FROM _datalib_meta WHERE key = ?",
                &[value.clone(), key.into()],
            )?;
            if same.as_i64() == Some(1) {
                continue;
            }
            self.conn.execute(
                "INSERT INTO _datalib_meta (key, value, written_at_utc, tz_offset) \
                 VALUES (?, ?, ?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, \
                 written_at_utc = excluded.written_at_utc, tz_offset = excluded.tz_offset",
                &[key.into(), value, now.clone().into(), offset.clone().into()],
            )?;
        }
        Ok(())
    }

    pub fn begin(&self) -> Result<()> {
        self.conn.execute("BEGIN", &[])
    }

    pub fn commit_transaction(&self) -> Result<()> {
        self.conn.execute("COMMIT", &[])
    }

    /// Commit what this run wrote and move `main` to it, where readers
    /// look. Nothing to commit is not an error.
    pub fn seal(&self, message: &str) -> Result<()> {
        let run = std::env::var("DATALIB_DAG_RUN_ID").unwrap_or_default();
        let message = if run.is_empty() {
            message.to_string()
        } else {
            format!("{message} run={run}")
        };
        if let Err(e) = self
            .conn
            .execute("SELECT dolt_commit('-Am', ?)", &[message.into()])
        {
            if !e.0.contains("nothing to commit") {
                return Err(e);
            }
        }
        self.publish()
    }

    /// Move `main` to the writer branch when it is behind.
    fn publish(&self) -> Result<()> {
        let same = self.conn.one(
            "SELECT dolt_hashof(?) = dolt_hashof('main')",
            &[WRITER_BRANCH.into()],
        );
        if matches!(same, Ok(Value::Int(1))) {
            return Ok(());
        }
        self.conn.execute(
            "SELECT dolt_branch('-f', 'main', ?)",
            &[WRITER_BRANCH.into()],
        )
    }

    /// The newest commit's hash.
    pub fn head_commit(&self) -> Result<String> {
        self.conn
            .one("SELECT commit_hash FROM dolt_log() LIMIT 1", &[])?
            .as_str()
            .map(String::from)
            .ok_or_else(|| Error("the store has no commit".into()))
    }

    /// The newest commit's tree hash: what a render step reports as the
    /// version of its store.
    pub fn head_tree(&self) -> Result<String> {
        self.conn
            .one(
                "SELECT dolt_hashof_db((SELECT commit_hash FROM dolt_log() LIMIT 1))",
                &[],
            )?
            .as_str()
            .map(String::from)
            .ok_or_else(|| Error("the store has no commit".into()))
    }
}

/// The lock datalib's own writers take, so a `--reset` or a second
/// writer is refused while this one runs.
fn take_lock(path: &Path) -> Result<File> {
    let mut name = path.as_os_str().to_owned();
    name.push(".lock");
    let lock_path = PathBuf::from(name);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| Error(format!("{}: {e}", lock_path.display())))?;
    file.try_lock()
        .map_err(|e| Error(format!("{} is held by another writer: {e}", path.display())))?;
    let _ = file.set_len(0);
    let _ = writeln!(
        file,
        "pid {} ({})",
        std::process::id(),
        std::env::args().next().unwrap_or_default()
    );
    Ok(file)
}
