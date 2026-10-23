//! A source's render store, `<group>/render_markdown/indexed_markdown.doltlite_db`,
//! and the `.md` files beside it: what datalib's `grid_index` reads into the
//! grid, and its keyword index and embeddings read for search.
//!
//! This writer is the simplest correct one: every run writes every
//! document. An unchanged document writes the same rows, which doltlite
//! stores as the same tree, so the index sees no change for it; a document
//! left out is deleted, and the index deletes it too.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::sqlite::Value;
use crate::store::{Store, StoreKind};
use crate::{ddl, step, Result};

pub const STORE_FILE: &str = "indexed_markdown.doltlite_db";

/// One document: one page in the grid, opened as its `.md` file.
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique across every source; mint it with [`crate::id::entity_id`].
    pub uuid: String,
    pub title: String,
    /// The grid's Kind column, at most 32 bytes.
    pub kind: String,
    /// RFC 3339 with an offset; a stamp without one is left out of date
    /// filters.
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub author: Option<String>,
    /// The body, as markdown.
    pub body: String,
    /// Where the file goes, relative to the step's tree: `greetings/x.md`.
    pub file: String,
    /// The record's own key upstream, and what kind of thing it is.
    pub upstream_id: Option<String>,
    pub upstream_entity_kind: Option<String>,
}

pub struct RenderStore {
    store: Store,
    tree: PathBuf,
    group: String,
    /// What the grid's Provider and Source columns say for this source.
    provider: String,
    source_label: String,
    /// Bumped when the package renders the same records differently.
    render_version: u32,
}

impl RenderStore {
    pub fn open(
        step: &step::Step,
        provider: &str,
        source_label: &str,
        render_version: u32,
    ) -> Result<RenderStore> {
        let tree = step.tree();
        let store = Store::open(
            &tree.join(STORE_FILE),
            StoreKind::Render,
            ddl::RENDER_DDL,
            ddl::RENDER_INDEXES,
        )?;
        Ok(RenderStore {
            store,
            tree,
            group: step.group.clone(),
            provider: provider.to_string(),
            source_label: source_label.to_string(),
            render_version,
        })
    }

    fn md_path(&self, doc: &Document) -> String {
        format!("{}/render_markdown/{}", self.group, doc.file)
    }

    /// Make the store hold exactly `docs`, publish it, and return its
    /// version: the tree hash a render step reports.
    pub fn write_all(&self, docs: &[Document]) -> Result<String> {
        let mut files = BTreeSet::new();
        for doc in docs {
            let path = self.tree.join(&doc.file);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| crate::Error(format!("{}: {e}", dir.display())))?;
            }
            std::fs::write(&path, self.markdown(doc))
                .map_err(|e| crate::Error(format!("{}: {e}", path.display())))?;
            files.insert(path);
        }
        let c = self.store.conn();
        self.store.begin()?;
        for table in [
            "grid_rows",
            "markdowns",
            "edges",
            "source_contacts",
            "source_contact_handles",
            "problems",
        ] {
            c.execute(&format!("DELETE FROM {table}"), &[])?;
        }
        for doc in docs {
            self.put(doc)?;
        }
        self.store.commit_transaction()?;
        self.store.seal(&format!(
            "render {}: {} document(s)",
            self.group,
            docs.len()
        ))?;
        remove_other_md_files(&self.tree, &files);
        self.store.head_tree()
    }

    /// Empty the store and its files: what `--reset store` asks.
    pub fn reset(&self) -> Result<String> {
        self.write_all(&[])
    }

    fn markdown(&self, doc: &Document) -> String {
        format!(
            "# {}\n\n<div id=\"m-{u}\" data-section-uuid=\"{u}\" class=\"msg msg--{p}\">\n\n{}\n\n</div>\n",
            doc.title,
            doc.body.trim_end(),
            u = doc.uuid,
            p = self.provider,
        )
    }

    fn put(&self, doc: &Document) -> Result<()> {
        let c = self.store.conn();
        let md_path = self.md_path(doc);
        c.execute(
            "INSERT INTO markdowns (markdown_uuid, source_id, provider, kind, title, created_at, \
             modified_at, item_count, md_path, upstream_cursor, renderer_version, bucket_key) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, NULL, ?, NULL)",
            &[
                (&doc.uuid).into(),
                (&self.group).into(),
                (&self.provider).into(),
                (&doc.kind).into(),
                (&doc.title).into(),
                doc.created_at.clone().into(),
                doc.modified_at.clone().into(),
                (&md_path).into(),
                format!("{}-v1.{}", self.provider, self.render_version).into(),
            ],
        )?;
        let (created_utc, created_offset) = split(doc.created_at.as_deref());
        let (modified_utc, modified_offset) = split(doc.modified_at.as_deref());
        c.execute(
            "INSERT INTO grid_rows (uuid, provider, kind, source_label, created_at, created_at_utc, \
             created_offset, modified_at, modified_at_utc, modified_offset, is_document, author, \
             conversation_name, conversation_uuid, entire_chat, preview, content_hash, qmd_path, \
             source_id, upstream_id, upstream_entity_kind, markdown_uuid, item_count) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
            &[
                (&doc.uuid).into(),
                (&self.provider).into(),
                (&doc.kind).into(),
                (&self.source_label).into(),
                doc.created_at.clone().into(),
                created_utc,
                created_offset,
                doc.modified_at.clone().into(),
                modified_utc,
                modified_offset,
                doc.author.clone().into(),
                (&doc.title).into(),
                (&doc.uuid).into(),
                format!("/chat/{}", doc.uuid).into(),
                preview(&doc.body).into(),
                blake3::hash(doc.body.as_bytes()).to_hex().to_string().into(),
                (&md_path).into(),
                (&self.group).into(),
                doc.upstream_id.clone().into(),
                doc.upstream_entity_kind.clone().into(),
                (&doc.uuid).into(),
            ],
        )
    }
}

/// The first 240 characters of the body as one line of plain text.
pub fn preview(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

/// A stamp's UTC twin and its offset, as the grid sorts and filters by.
fn split(stamp: Option<&str>) -> (Value, Value) {
    let Some(at) = stamp.and_then(step::parse_time) else {
        return (Value::Null, Value::Null);
    };
    let utc = at
        .to_offset(time::UtcOffset::UTC)
        .format(&time::format_description::well_known::Rfc3339)
        .ok();
    let (h, m, _) = at.offset().as_hms();
    let sign = if at.offset().is_negative() { '-' } else { '+' };
    (
        utc.into(),
        format!("{sign}{:02}:{:02}", h.abs(), m.abs()).into(),
    )
}

fn remove_other_md_files(dir: &Path, keep: &BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_other_md_files(&path, keep);
        } else if path.extension().is_some_and(|e| e == "md") && !keep.contains(&path) {
            let _ = std::fs::remove_file(&path);
        }
    }
}
