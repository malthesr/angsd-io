//! Reading and writing of the GLF format.
//!
//! # Examples
//!
//! Read BGZF GLF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/glf_read.rs"))]
//! ```
//!
//! Write BGZF GLF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/glf_write.rs"))]
//! ```
//!
//! The above examples are also available as runnable binaries,
//! see the repository `examples/` folder.

pub(self) type Endian = byteorder::LittleEndian;

mod reader;
pub use reader::{BgzfReader, Reader};

pub mod record;
pub use record::{Genotype, Record};

mod writer;
pub use writer::{BgzfWriter, Writer};
