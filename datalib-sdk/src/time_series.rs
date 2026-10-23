//! A source's time-series store, `<group>/render_time_series/time_series.doltlite_db`:
//! what datalib's `time_series_index` lists and its Time series card
//! charts. Every run writes every series; an unchanged one writes the same
//! rows. The chunk encoding is copied from datalib's
//! `etl/time_series/src/chunk.rs` (MIT, Copyright imbue-ai), the plain
//! layouts only: integers as deltas, floats as raw little-endian f64.

use crate::sqlite::Value;
use crate::store::{Store, StoreKind};
use crate::{ddl, step, Result};

pub const STORE_FILE: &str = "time_series.doltlite_db";

/// Points per chunk, as datalib cuts them.
const CHUNK_POINTS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeKind {
    /// `t` is unix milliseconds.
    Instant,
    /// `t` is days since 1970-01-01, a calendar day as the person lived it.
    Day,
}

#[derive(Debug, Clone)]
pub enum Values {
    Integer(Vec<i64>),
    Float(Vec<f64>),
}

impl Values {
    fn len(&self) -> usize {
        match self {
            Values::Integer(v) => v.len(),
            Values::Float(v) => v.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Unique across every source; mint it with [`crate::id::entity_id`].
    pub time_series_id: String,
    /// The document it is about, and what a list calls that document; none
    /// for one about the source as a whole.
    pub about: Option<(String, String)>,
    /// The package's key for what is measured, unique within the source.
    pub name: String,
    pub human_name: String,
    /// A UCUM code; `1` for a count.
    pub unit: String,
    pub time_kind: TimeKind,
    /// Strictly ascending.
    pub t: Vec<i64>,
    pub values: Values,
}

pub struct TimeSeriesStore {
    store: Store,
    group: String,
}

impl TimeSeriesStore {
    pub fn open(step: &step::Step) -> Result<TimeSeriesStore> {
        let store = Store::open(
            &step.tree().join(STORE_FILE),
            StoreKind::TimeSeries,
            ddl::TIME_SERIES_DDL,
            ddl::TIME_SERIES_INDEXES,
        )?;
        Ok(TimeSeriesStore {
            store,
            group: step.group.clone(),
        })
    }

    /// Make the store hold exactly `series`, publish it, and return its
    /// version: the commit hash a time-series step reports.
    pub fn write_all(&self, series: &[TimeSeries]) -> Result<String> {
        let c = self.store.conn();
        self.store.begin()?;
        c.execute("DELETE FROM time_series", &[])?;
        c.execute("DELETE FROM time_series_chunks", &[])?;
        for (i, s) in series.iter().enumerate() {
            let ts_key = i as i64 + 1;
            let n = s.t.len();
            if s.values.len() != n {
                return Err(crate::Error(format!(
                    "time series {}: {} times but {} values",
                    s.name,
                    n,
                    s.values.len()
                )));
            }
            let (value_type, time_kind) = (
                match s.values {
                    Values::Integer(_) => "integer",
                    Values::Float(_) => "float",
                },
                match s.time_kind {
                    TimeKind::Instant => "instant",
                    TimeKind::Day => "day",
                },
            );
            c.execute(
                "INSERT INTO time_series (source_id, ts_key, time_series_id, about_uuid, \
                 about_name, name, human_name, unit, value_type, time_kind, point_count, \
                 first_t, last_t) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    (&self.group).into(),
                    ts_key.into(),
                    (&s.time_series_id).into(),
                    s.about.as_ref().map(|a| a.0.clone()).into(),
                    s.about.as_ref().map(|a| a.1.clone()).into(),
                    (&s.name).into(),
                    (&s.human_name).into(),
                    (&s.unit).into(),
                    value_type.into(),
                    time_kind.into(),
                    (n as i64).into(),
                    s.t.first().copied().into(),
                    s.t.last().copied().into(),
                ],
            )?;
            for start in (0..n).step_by(CHUNK_POINTS) {
                let end = (start + CHUNK_POINTS).min(n);
                let t = &s.t[start..end];
                let (blob, numbers) = match &s.values {
                    Values::Integer(v) => {
                        let v = &v[start..end];
                        (
                            encode(t, Part::Integer(v)),
                            v.iter().map(|&x| x as f64).collect(),
                        )
                    }
                    Values::Float(v) => {
                        let v = &v[start..end];
                        (encode(t, Part::Float(v)), v.to_vec())
                    }
                };
                let numbers: Vec<f64> = numbers;
                c.execute(
                    "INSERT INTO time_series_chunks (ts_key, first_t, last_t, n, min, max, sum, \
                     encoding, points) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?)",
                    &[
                        ts_key.into(),
                        t[0].into(),
                        t[t.len() - 1].into(),
                        (t.len() as i64).into(),
                        numbers.iter().copied().fold(f64::INFINITY, f64::min).into(),
                        numbers
                            .iter()
                            .copied()
                            .fold(f64::NEG_INFINITY, f64::max)
                            .into(),
                        numbers.iter().sum::<f64>().into(),
                        Value::Blob(blob),
                    ],
                )?;
            }
        }
        self.store.commit_transaction()?;
        self.store.seal(&format!(
            "time series {}: {} series",
            self.group,
            series.len()
        ))?;
        self.store.head_commit()
    }
}

enum Part<'a> {
    Integer(&'a [i64]),
    Float(&'a [f64]),
}

const PLAIN: u8 = 0;
const INTEGER: u8 = 0;
const RAW_FLOAT: u8 = 2;

fn encode(t: &[i64], values: Part<'_>) -> Vec<u8> {
    let mut out = vec![PLAIN];
    varint(t.len() as u64, &mut out);
    times(t, &mut out);
    match values {
        Part::Integer(v) => {
            out.push(INTEGER);
            deltas(v, &mut out);
        }
        Part::Float(v) => {
            out.push(RAW_FLOAT);
            for x in v {
                out.extend(x.to_le_bytes());
            }
        }
    }
    out
}

fn zigzag(x: i64) -> u64 {
    ((x << 1) ^ (x >> 63)) as u64
}

fn varint(mut x: u64, out: &mut Vec<u8>) {
    while x >= 0x80 {
        out.push((x as u8) | 0x80);
        x >>= 7;
    }
    out.push(x as u8);
}

/// Every value at the width of the widest: a byte for the width, then
/// the bits, least significant first.
fn pack(xs: &[u64], out: &mut Vec<u8>) {
    let width = xs.iter().map(|x| 64 - x.leading_zeros()).max().unwrap_or(0);
    out.push(width as u8);
    if width == 0 {
        return;
    }
    let mut acc: u128 = 0;
    let mut bits = 0;
    for &x in xs {
        acc |= u128::from(x) << bits;
        bits += width;
        while bits >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    }
    if bits > 0 {
        out.push(acc as u8);
    }
}

fn times(t: &[i64], out: &mut Vec<u8>) {
    let Some(&first) = t.first() else { return };
    varint(zigzag(first), out);
    if t.len() < 2 {
        return;
    }
    let gaps: Vec<i64> = t.windows(2).map(|w| w[1].wrapping_sub(w[0])).collect();
    varint(zigzag(gaps[0]), out);
    let changes: Vec<u64> = gaps
        .windows(2)
        .map(|g| zigzag(g[1].wrapping_sub(g[0])))
        .collect();
    pack(&changes, out);
}

fn deltas(v: &[i64], out: &mut Vec<u8>) {
    let Some(&first) = v.first() else { return };
    varint(zigzag(first), out);
    let changes: Vec<u64> = v
        .windows(2)
        .map(|w| zigzag(w[1].wrapping_sub(w[0])))
        .collect();
    pack(&changes, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes datalib's encoder writes for the same points
    /// (`chunk::encode` with no compression).
    #[test]
    fn a_short_integer_series_encodes_as_datalib_does() {
        let blob = encode(&[1000, 2000, 3000], Part::Integer(&[5, 6, 8]));
        // plain, n=3, t0=zz(1000)=2000 as varint, gap zz(1000)=2000, changes
        // packed at width 0; INTEGER, v0=zz(5)=10, changes zz(1)=2, zz(2)=4
        // packed 3 bits wide: 0b100_010 = 0x22.
        assert_eq!(blob, vec![0, 3, 0xd0, 0x0f, 0xd0, 0x0f, 0, 0, 10, 3, 0x22]);
    }
}
