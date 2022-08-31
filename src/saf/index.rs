//! Reading and writing of the SAF index format.

use std::{fmt, io, path::Path};

use super::{Version, V3};

mod reader;
pub use reader::Reader;

mod record;
pub use record::Record;

mod writer;
pub use writer::Writer;

/// A SAF file index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index<V: Version = V3> {
    alleles: usize,
    records: Vec<Record<V>>,
}

impl<V> Index<V>
where
    V: Version,
{
    /// Returns the number of alleles.
    ///
    /// This is equal to `2N` for `N` diploid individuals.
    pub fn alleles(&self) -> usize {
        self.alleles
    }

    /// Returns a mutable reference to the number of alleles.
    ///
    /// This is equal to `2N` for `N` diploid individuals.
    pub fn alleles_mut(&mut self) -> &mut usize {
        &mut self.alleles
    }

    /// Returns the index records, consuming `self`.
    pub fn into_records(self) -> Vec<Record<V>> {
        self.records
    }

    /// Creates a new index.
    pub fn new(alleles: usize, records: Vec<Record<V>>) -> Self {
        Self { alleles, records }
    }

    /// Returns the index records.
    pub fn records(&self) -> &[Record<V>] {
        self.records.as_ref()
    }

    /// Returns a mutable reference to the index records.
    pub fn records_mut(&mut self) -> &mut Vec<Record<V>> {
        &mut self.records
    }

    /// Returns the total number of sites.
    pub fn total_sites(&self) -> usize {
        self.records.iter().map(|rec| rec.sites()).sum()
    }
}

impl Index<V3> {
    /// Creates a new index by reading from a path.
    pub fn read_from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Reader::<_, V3>::from_path(path).and_then(|mut reader| reader.read_index())
    }

    /// Writes the index to a path.
    ///
    /// If `path` already exists, it will be overwritten.
    pub fn write_to_path<P>(&self, path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        Writer::<_, V3>::from_path(path).and_then(|mut writer| writer.write_index(self))
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "##alleles={}", self.alleles)?;

        for record in self.records() {
            write!(f, "{}", record)?;
        }

        Ok(())
    }
}
