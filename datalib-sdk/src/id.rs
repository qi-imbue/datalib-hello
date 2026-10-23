//! Ids in datalib's layout (its docs/dev/entity_ids.md): a UUIDv5 over
//! the recipe, with the record's time in the first 48 bits so ids sort by
//! time, marked version 8. Copied from datalib's `id/src/lib.rs` (MIT,
//! Copyright imbue-ai).

const NAMESPACE: &[u8; 16] = b"datalib-id-ns-v1";

/// The id of one record. `namespace` keeps a package's ids apart from
/// every other's (use the package's own name, not one of datalib's
/// providers); `source_id` is the group id, so two sources never share
/// an id; `at_ms` is the record's creation time, when it has one.
pub fn entity_id(
    namespace: &str,
    source_id: &str,
    account: Option<&str>,
    entity_kind: &str,
    natural_key: &str,
    at_ms: Option<i64>,
) -> String {
    let recipe = [
        namespace,
        source_id,
        account.unwrap_or(""),
        entity_kind,
        natural_key,
    ]
    .join("\x1f");
    let mut sha = sha1_smol::Sha1::new();
    sha.update(NAMESPACE);
    sha.update(recipe.as_bytes());
    let digest = sha.digest().bytes();
    let mut b = [0u8; 16];
    b.copy_from_slice(&digest[..16]);
    b[6] = (b[6] & 0x0f) | 0x50;
    b[8] = (b[8] & 0x3f) | 0x80;
    let ms = at_ms.unwrap_or(0).clamp(0, (1 << 48) - 1) as u64;
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[6] = 0x80 | (b[6] & 0x0f);
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// A key made of several parts, each escaped so no two lists join to the
/// same string.
pub fn composite_key(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|p| p.replace('%', "%25").replace('#', "%23"))
        .collect::<Vec<_>>()
        .join("#")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stamp_leads_and_the_version_is_8() {
        let id = entity_id("hello", "h", None, "greeting", "k", Some(0x0123_4567_89ab));
        assert!(id.starts_with("01234567-89ab-8"), "{id}");
        assert_eq!(id.len(), 36);
        assert_ne!(
            id,
            entity_id("hello", "h2", None, "greeting", "k", Some(0x0123_4567_89ab))
        );
    }
}
