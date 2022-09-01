//! The SAF index format.

use std::{fmt, fs, io, path::Path};

use crate::saf::reader::ReaderExt;

use super::Version;

mod record;
pub use record::Record;

mod traits;
pub(in crate::saf) use traits::{IndexReaderExt, IndexWriterExt};

/// A SAF file index.
///
/// Different SAF file versions differ only in what their records contain. For more details. see
/// [`Record`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index<V> {
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

    /// Reads a new index from a reader.
    ///
    /// The stream is assumed to be positioned at the start.
    pub fn read<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        V::read_magic(reader)?;

        let alleles = reader.read_alleles()?;

        let mut records = Vec::new();
        while reader.is_data_left()? {
            let record = Record::read(reader)?;

            records.push(record)
        }

        Ok(Index::new(alleles, records))
    }

    /// Creates a new index by reading from a path.
    pub fn read_from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::open(path)
            .map(io::BufReader::new)
            .and_then(|mut reader| Self::read(&mut reader))
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

    /// Writes the index to a writer.
    pub fn write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        V::write_magic(writer)?;

        writer.write_alleles(self.alleles())?;

        for record in self.records() {
            record.write(writer)?;
        }

        Ok(())
    }

    /// Writes the index to a path.
    ///
    /// If `path` already exists, it will be overwritten.
    pub fn write_to_path<P>(&self, path: P) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        fs::File::create(path)
            .map(io::BufWriter::new)
            .and_then(|mut writer| self.write(&mut writer))
    }
}

impl<V> fmt::Display for Index<V>
where
    V: Version,
    Record<V>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "##version=v{}", V::VERSION)?;
        writeln!(f, "##alleles={}", self.alleles)?;

        for record in self.records() {
            write!(f, "{}", record)?;
        }

        Ok(())
    }
}
