//! Write the stores a datalib source's steps produce, from a program
//! outside datalib: the render store and `.md` files datalib's document
//! index and search read, and the time-series store its time-series index
//! reads. A package's render steps use this the way datalib's own render
//! steps use its internal crates.
//!
//! The formats are datalib's, copied from datalib 0.44.0; a datalib whose
//! render-store schema hash differs reads none of these stores until this
//! crate is updated to match.

pub mod ddl;
pub mod id;
pub mod render;
pub mod sqlite;
pub mod step;
pub mod store;
pub mod time_series;

pub use sqlite::{Error, Result};

#[cfg(test)]
mod tests {
    use crate::{ddl, store};

    /// The hashes datalib 0.44.0 computes for the same DDL.
    #[test]
    fn the_schema_hashes_are_datalibs() {
        assert_eq!(store::schema_hash(ddl::RENDER_DDL), ddl::RENDER_SCHEMA_HASH);
        assert_eq!(
            store::schema_hash(ddl::TIME_SERIES_DDL),
            ddl::TIME_SERIES_SCHEMA_HASH
        );
    }
}
