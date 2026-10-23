//! The Hello source's `render_time_series`: how long each greeting was,
//! and how many there were each day, in the store datalib's time-series
//! index reads.

use std::collections::BTreeMap;

use datalib_sdk::time_series::{TimeKind, TimeSeries, TimeSeriesStore, Values};
use datalib_sdk::{id, step};

#[path = "../greetings.rs"]
mod greetings;

/// Julian day number of 1970-01-01.
const UNIX_EPOCH_JULIAN_DAY: i64 = 2_440_588;

fn main() {
    greetings::run_step("hello-time-series", || {
        let step = step::Step::from_env().map_err(|e| e.to_string())?;
        let store = TimeSeriesStore::open(&step).map_err(|e| e.to_string())?;
        let ingest = step.data_root.join(format!("{}/ingest", step.group));
        let (greetings, bad) = if step.reset {
            (Vec::new(), 0)
        } else {
            greetings::read(&ingest)
        };

        // Times must be strictly ascending: a later greeting in the same
        // millisecond replaces an earlier one.
        let mut lengths: BTreeMap<i64, i64> = BTreeMap::new();
        let mut per_day: BTreeMap<i64, i64> = BTreeMap::new();
        for g in &greetings {
            let at = step::parse_time(&g.at).expect("read() keeps only stamps that parse");
            lengths.insert(
                step::unix_ms(&g.at).unwrap(),
                g.greeting.chars().count() as i64,
            );
            *per_day
                .entry(i64::from(at.date().to_julian_day()) - UNIX_EPOCH_JULIAN_DAY)
                .or_default() += 1;
        }
        let series_id = |name: &str| {
            id::entity_id(
                "hello",
                &step.group,
                None,
                "time_series",
                &id::composite_key(&[name]),
                None,
            )
        };
        let series = vec![
            TimeSeries {
                time_series_id: series_id("greeting_length"),
                about: None,
                name: "greeting_length".into(),
                human_name: "Greeting length".into(),
                unit: "1".into(),
                time_kind: TimeKind::Instant,
                t: lengths.keys().copied().collect(),
                values: Values::Integer(lengths.values().copied().collect()),
            },
            TimeSeries {
                time_series_id: series_id("greetings_per_day"),
                about: None,
                name: "greetings_per_day".into(),
                human_name: "Greetings per day".into(),
                unit: "1".into(),
                time_kind: TimeKind::Day,
                t: per_day.keys().copied().collect(),
                values: Values::Integer(per_day.values().copied().collect()),
            },
        ];
        let version = store.write_all(&series).map_err(|e| e.to_string())?;
        step::metric(
            &step.id,
            "problems",
            Some(serde_json::json!({"severity": "error"})),
            0,
        );
        step::metric(
            &step.id,
            "problems",
            Some(serde_json::json!({"severity": "warning"})),
            bad as i64,
        );
        step::outcome(&step.id, &version, None);
        Ok(())
    });
}
