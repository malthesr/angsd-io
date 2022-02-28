use std::{fs, io, path::Path};

use crate::saf::write_magic;

use super::{Index, Record};

/// A SAF index writer.
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    /// Returns the inner writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns the inner writer.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns the inner writer, consuming `self.`
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// Creates a new writer.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Writes the alleles.
    pub fn write_alleles(&mut self, alleles: usize) -> io::Result<()> {
        self.inner.write_all(&alleles.to_le_bytes())
    }

    /// Writes an entire index.
    ///
    /// See also the [`Index::write_to_path`] convenience method.
    pub fn write_index(&mut self, index: &Index) -> io::Result<()> {
        self.write_magic()?;

        self.write_alleles(index.alleles())?;

        for record in index.records() {
            self.write_record(record)?;
        }

        Ok(())
    }

    /// Writes the magic number.
    pub fn write_magic(&mut self) -> io::Result<()> {
        write_magic(&mut self.inner)
    }

    /// Writes a single record.
    pub fn write_record(&mut self, record: &Record) -> io::Result<()> {
        let name = record.name().as_bytes();

        let writer = &mut self.inner;
        writer.write_all(&name.len().to_le_bytes())?;
        writer.write_all(name)?;
        writer.write_all(&record.sites().to_le_bytes())?;
        writer.write_all(&record.position_offset().to_le_bytes())?;
        writer.write_all(&record.value_offset().to_le_bytes())?;

        Ok(())
    }
}

impl Writer<io::BufWriter<fs::File>> {
    /// Creates a new writer from a path.
    ///
    /// If the path already exists, it will be overwritten.
    ///
    /// The magic number will be written to the path.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut writer = fs::File::create(path)
            .map(io::BufWriter::new)
            .map(Self::new)?;

        writer.write_magic()?;

        Ok(writer)
    }
}

impl<W> From<W> for Writer<W>
where
    W: io::Write,
{
    fn from(inner: W) -> Self {
        Self::new(inner)
    }
}
