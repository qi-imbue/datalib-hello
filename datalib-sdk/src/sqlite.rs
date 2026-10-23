//! The few SQLite calls this crate makes, against the doltlite build that
//! `build.rs` compiles in. Doltlite is SQLite's API over a different
//! storage engine, so these are the stock `sqlite3_*` functions.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;

#[allow(non_camel_case_types)]
type sqlite3 = c_void;
#[allow(non_camel_case_types)]
type sqlite3_stmt = c_void;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OPEN_READWRITE: c_int = 0x02;
const SQLITE_OPEN_CREATE: c_int = 0x04;
const SQLITE_OPEN_FULLMUTEX: c_int = 0x10000;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_TEXT: c_int = 3;
const SQLITE_BLOB: c_int = 4;

// SQLITE_TRANSIENT: SQLite copies the bound bytes before the call returns.
fn transient() -> Option<unsafe extern "C" fn(*mut c_void)> {
    unsafe { std::mem::transmute::<isize, Option<unsafe extern "C" fn(*mut c_void)>>(-1) }
}

extern "C" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        db: *mut *mut sqlite3,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close_v2(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_busy_timeout(db: *mut sqlite3, ms: c_int) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        sql: *const c_char,
        n: c_int,
        stmt: *mut *mut sqlite3_stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_bind_null(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, i: c_int, v: i64) -> c_int;
    fn sqlite3_bind_double(stmt: *mut sqlite3_stmt, i: c_int, v: f64) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut sqlite3_stmt,
        i: c_int,
        v: *const c_char,
        n: c_int,
        d: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_bind_blob(
        stmt: *mut sqlite3_stmt,
        i: c_int,
        v: *const c_void,
        n: c_int,
        d: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_column_count(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_type(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, i: c_int) -> i64;
    fn sqlite3_column_double(stmt: *mut sqlite3_stmt, i: c_int) -> f64;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, i: c_int) -> *const u8;
    fn sqlite3_column_blob(stmt: *mut sqlite3_stmt, i: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, i: c_int) -> c_int;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string())
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}
impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Value::Text(s.clone())
    }
}
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Real(f)
    }
}
impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Blob(b)
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        o.map_or(Value::Null, Into::into)
    }
}

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// One connection. Not `Sync`: a store has one writer, on one thread.
pub struct Connection {
    db: *mut sqlite3,
}

impl Connection {
    pub fn open(path: &Path) -> Result<Connection> {
        let c = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| Error(format!("{} has a NUL in it", path.display())))?;
        let mut db = ptr::null_mut();
        let flags = SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX;
        let rc = unsafe { sqlite3_open_v2(c.as_ptr(), &mut db, flags, ptr::null()) };
        let conn = Connection { db };
        if rc != SQLITE_OK {
            return Err(Error(format!("open {}: {}", path.display(), conn.errmsg())));
        }
        unsafe { sqlite3_busy_timeout(db, 30_000) };
        Ok(conn)
    }

    fn errmsg(&self) -> String {
        if self.db.is_null() {
            return "out of memory".into();
        }
        unsafe { CStr::from_ptr(sqlite3_errmsg(self.db)) }
            .to_string_lossy()
            .into_owned()
    }

    /// Run `sql` with `params` bound in order, and return every row.
    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<Value>>> {
        let c = CString::new(sql).map_err(|_| Error("SQL with a NUL in it".into()))?;
        let mut stmt = ptr::null_mut();
        let rc = unsafe { sqlite3_prepare_v2(self.db, c.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(Error(format!("{sql}: {}", self.errmsg())));
        }
        let stmt = Stmt(stmt);
        for (i, p) in params.iter().enumerate() {
            let i = i as c_int + 1;
            let rc = unsafe {
                match p {
                    Value::Null => sqlite3_bind_null(stmt.0, i),
                    Value::Int(v) => sqlite3_bind_int64(stmt.0, i, *v),
                    Value::Real(v) => sqlite3_bind_double(stmt.0, i, *v),
                    Value::Text(s) => sqlite3_bind_text(
                        stmt.0,
                        i,
                        s.as_ptr().cast(),
                        s.len() as c_int,
                        transient(),
                    ),
                    Value::Blob(b) => sqlite3_bind_blob(
                        stmt.0,
                        i,
                        b.as_ptr().cast(),
                        b.len() as c_int,
                        transient(),
                    ),
                }
            };
            if rc != SQLITE_OK {
                return Err(Error(format!("{sql}: bind {i}: {}", self.errmsg())));
            }
        }
        let mut rows = Vec::new();
        loop {
            match unsafe { sqlite3_step(stmt.0) } {
                SQLITE_DONE => return Ok(rows),
                SQLITE_ROW => rows.push(read_row(stmt.0)),
                _ => return Err(Error(format!("{sql}: {}", self.errmsg()))),
            }
        }
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<()> {
        self.query(sql, params).map(|_| ())
    }

    /// The first column of the first row, or `Null` when there is none.
    pub fn one(&self, sql: &str, params: &[Value]) -> Result<Value> {
        Ok(self
            .query(sql, params)?
            .into_iter()
            .next()
            .and_then(|r| r.into_iter().next())
            .unwrap_or(Value::Null))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe { sqlite3_close_v2(self.db) };
    }
}

struct Stmt(*mut sqlite3_stmt);

impl Drop for Stmt {
    fn drop(&mut self) {
        unsafe { sqlite3_finalize(self.0) };
    }
}

fn read_row(stmt: *mut sqlite3_stmt) -> Vec<Value> {
    let n = unsafe { sqlite3_column_count(stmt) };
    (0..n)
        .map(|i| unsafe {
            match sqlite3_column_type(stmt, i) {
                SQLITE_INTEGER => Value::Int(sqlite3_column_int64(stmt, i)),
                SQLITE_FLOAT => Value::Real(sqlite3_column_double(stmt, i)),
                SQLITE_TEXT => {
                    let p = sqlite3_column_text(stmt, i);
                    let len = sqlite3_column_bytes(stmt, i) as usize;
                    let bytes = if p.is_null() {
                        &[][..]
                    } else {
                        std::slice::from_raw_parts(p, len)
                    };
                    Value::Text(String::from_utf8_lossy(bytes).into_owned())
                }
                SQLITE_BLOB => {
                    let p = sqlite3_column_blob(stmt, i) as *const u8;
                    let len = sqlite3_column_bytes(stmt, i) as usize;
                    let bytes = if p.is_null() {
                        &[][..]
                    } else {
                        std::slice::from_raw_parts(p, len)
                    };
                    Value::Blob(bytes.to_vec())
                }
                _ => Value::Null,
            }
        })
        .collect()
}
