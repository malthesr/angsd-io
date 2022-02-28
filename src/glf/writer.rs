use std::{fs, io, path::Path};

use byteorder::WriteBytesExt;

use super::{Endian, Record};

/// A BGZF GLF writer.
///
/// Note that this is a type alias for a [`Writer`], and most methods are
/// available via the [`Writer`] type.
pub type BgzfWriter<W> = Writer<bgzf::Writer<W>>;

/// A GLF writer
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

    /// Writes a single record.
    pub fn write_record(&mut self, record: &Record) -> io::Result<()> {
        for value in record.as_ref().iter() {
            self.inner.write_f64::<Endian>(*value)?;
        }

        Ok(())
    }

    /// Writes multiple records.
    pub fn write_records(&mut self, records: &[Record]) -> io::Result<()> {
        for record in records {
            self.write_record(record)?;
        }

        Ok(())
    }
}

impl Writer<io::BufWriter<fs::File>> {
    /// Creates a new writer from a path.
    ///
    /// Note that the constructed writer will not be a BGZF writer.
    /// To construct a BGZF writer from path, see the
    /// [`BgzfWriter::from_bgzf_path`] constructor.
    ///
    /// If the path already exists, it will be overwritten.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::create(path)
            .map(io::BufWriter::new)
            .map(Self::new)
    }
}

impl<W> BgzfWriter<W>
where
    W: io::Write,
{
    /// Creates a new BGZF writer.
    pub fn from_bgzf(inner: W) -> Self {
        Self::new(bgzf::Writer::new(inner))
    }
}

impl BgzfWriter<io::BufWriter<fs::File>> {
    /// Creates a new BGZF writer from a path.
    ///
    /// If the path already exists, it will be overwritten.
    pub fn from_bgzf_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::create(path)
            .map(io::BufWriter::new)
            .map(Self::from_bgzf)
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
