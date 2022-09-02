//! Reading and writing of the SAF format.
//!
//! # Examples
//!
//! Read BGZF SAF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_read.rs"))]
//! ```
//!
//! Write BGZF SAF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_write.rs"))]
//! ```
//!
//! Read only intersecting sites in multiple BGZF SAF files:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_intersect.rs"))]
//! ```
//!
//! The above examples are also available as runnable binaries,
//! see the repository `examples/` folder.

pub use angsd_io_core::ReadStatus;

pub mod ext;

pub mod index;
pub use index::Index;

mod reader;
pub use reader::{Intersect, Reader, ReaderV3, ReaderV4};

pub mod record;
pub use record::Record;

pub mod version;

mod writer;
pub use writer::{Writer, WriterV3, WriterV4};
