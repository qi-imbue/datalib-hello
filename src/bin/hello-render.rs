//! The Hello source's `render_markdown`: one document per greeting, in
//! the render store datalib's document index and search read.

use datalib_sdk::render::{Document, RenderStore};
use datalib_sdk::{id, step};

#[path = "../greetings.rs"]
mod greetings;

/// Bump when the same greetings render differently.
const RENDER_VERSION: u32 = 1;

fn main() {
    greetings::run_step("hello-render", || {
        let step = step::Step::from_env().map_err(|e| e.to_string())?;
        let store = RenderStore::open(&step, "hello", "Hello", RENDER_VERSION)
            .map_err(|e| e.to_string())?;
        if step.reset {
            let version = store.reset().map_err(|e| e.to_string())?;
            step::no_problems(&step.id);
            step::outcome(&step.id, &version, Some(0));
            return Ok(());
        }
        let ingest = step.data_root.join(format!("{}/ingest", step.group));
        let (greetings, bad) = greetings::read(&ingest);
        let docs: Vec<Document> = greetings
            .iter()
            .map(|g| {
                let uuid = id::entity_id(
                    "hello",
                    &step.group,
                    None,
                    "greeting",
                    &g.at,
                    step::unix_ms(&g.at),
                );
                Document {
                    title: g.greeting.chars().take(80).collect(),
                    kind: "greeting".into(),
                    created_at: Some(g.at.clone()),
                    modified_at: None,
                    author: None,
                    body: format!(
                        "{}\n\nWritten by the Hello source `{}` at {}.",
                        g.greeting, step.group, g.at
                    ),
                    file: format!("greetings/{uuid}.md"),
                    upstream_id: Some(g.at.clone()),
                    upstream_entity_kind: Some("greeting".into()),
                    uuid,
                }
            })
            .collect();
        let version = store.write_all(&docs).map_err(|e| e.to_string())?;
        let n = docs.len() as i64;
        step::metric(&step.id, "documents", None, n);
        step::metric(&step.id, "items", None, n);
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
        step::outcome(&step.id, &version, Some(n));
        Ok(())
    });
}
