//! Reading and writing of various file formats associated with
//! [ANGSD](https://github.com/angsd/angsd).
//!
//! See the individuals modules for examples.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod glf;
pub mod saf;

pub(crate) mod read;
pub use read::ReadStatus;
