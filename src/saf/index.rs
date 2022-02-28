//! Reading and writing of the SAF index format.

use std::{fmt, io, path::Path};

mod reader;
pub use reader::Reader;

mod record;
pub use record::Record;

mod writer;
pub use writer::Writer;

/// A SAF file index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index {
    alleles: usize,
    records: Vec<Record>,
}

impl Index {
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

    /// Creates a new index by reading from a path.
    pub fn read_from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Reader::from_path(path).and_then(|mut reader| reader.read_index())
    }

    /// Returns the index records, consuming `self`.
    pub fn into_records(self) -> Vec<Record> {
        self.records
    }

    /// Creates a new index.
    pub fn new(alleles: usize, records: Vec<Record>) -> Self {
        Self { alleles, records }
    }

    /// Returns the index records.
    pub fn records(&self) -> &[Record] {
        self.records.as_ref()
    }

    /// Returns a mutable reference to the index records.
    pub fn records_mut(&mut self) -> &mut Vec<Record> {
        &mut self.records
    }

    /// Returns the total number of sites.
    pub fn total_sites(&self) -> usize {
        self.records.iter().map(|rec| rec.sites()).sum()
    }

    /// Writes the index to a path.
    ///
    /// If `path` already exists, it will be overwritten.
    pub fn write_to_path<P>(&self, path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        Writer::from_path(path).and_then(|mut writer| writer.write_index(self))
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
