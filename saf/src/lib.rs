//! Reading and writing of the SAF format.
//!
//! # Examples
//!
//! Read SAF v3 file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_v3_read.rs"))]
//! ```
//!
//! Write SAF v3 file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_v3_write.rs"))]
//! ```
//!
//! Read only intersecting sites in multiple SAF v3 files:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_v3_intersect.rs"))]
//! ```
//!
//! The above examples are also available as runnable binaries,
//! see the repository `examples/` folder.

pub use angsd_io_core::ReadStatus;

pub mod ext;

pub mod index;
pub use index::Index;

pub mod reader;
pub use reader::{Intersect, Reader, ReaderV3, ReaderV4};

pub mod record;
pub use record::Record;

pub mod version;

pub mod writer;
pub use writer::{Writer, WriterV3, WriterV4};
