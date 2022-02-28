use std::{fs, io, path::Path};

use byteorder::WriteBytesExt;

use crate::saf::{write_magic, Endian};

/// A BGZF SAF value writer.
///
/// Note that this is a type alias for a [`ValueWriter`], and most methods are
/// available via the [`ValueWriter`] type.
pub type BgzfValueWriter<W> = ValueWriter<bgzf::Writer<W>>;

/// A SAF value writer.
pub struct ValueWriter<W> {
    inner: W,
}

impl<W> ValueWriter<W>
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

    /// Writes a single value.
    pub fn write_value(&mut self, value: f32) -> io::Result<()> {
        self.inner.write_f32::<Endian>(value)
    }

    /// Writes multiple values.
    pub fn write_values(&mut self, values: &[f32]) -> io::Result<()> {
        for value in values {
            self.write_value(*value)?
        }

        Ok(())
    }

    /// Writes the magic number.
    pub fn write_magic(&mut self) -> io::Result<()> {
        write_magic(&mut self.inner)
    }
}

impl ValueWriter<io::BufWriter<fs::File>> {
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

impl<W> BgzfValueWriter<W>
where
    W: io::Write,
{
    /// Creates a new BGZF writer.
    pub fn from_bgzf(inner: W) -> Self {
        Self::new(bgzf::Writer::new(inner))
    }
}

impl BgzfValueWriter<io::BufWriter<fs::File>> {
    /// Creates a new BGZF writer from a path.
    ///
    /// If the path already exists, it will be overwritten.
    ///
    /// The magic number will be written to the path.
    pub fn from_bgzf_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut writer = fs::File::create(path)
            .map(io::BufWriter::new)
            .map(Self::from_bgzf)?;

        writer.write_magic()?;

        Ok(writer)
    }
}

impl<W> From<W> for ValueWriter<W>
where
    W: io::Write,
{
    fn from(inner: W) -> Self {
        Self::new(inner)
    }
}
