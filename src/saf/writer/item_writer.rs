use std::{fs, io, path::Path};

use byteorder::WriteBytesExt;

use crate::saf::{Endian, Version, V3};

/// A BGZF SAF item writer.
///
/// Note that this is a type alias for a [`ItemWriter`], and most methods are
/// available via the [`ItemWriter`] type.
pub type BgzfItemWriter<W> = ItemWriter<bgzf::Writer<W>>;

/// A SAF item writer.
pub struct ItemWriter<W> {
    inner: W,
}

impl<W> ItemWriter<W>
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

    /// Writes a single float.
    fn write_float(&mut self, v: f32) -> io::Result<()> {
        self.inner.write_f32::<Endian>(v)
    }

    /// Writes a single item.
    pub fn write_item(&mut self, item: &[f32]) -> io::Result<()> {
        for v in item {
            self.write_float(*v)?
        }

        Ok(())
    }

    /// Writes the magic number.
    pub fn write_magic(&mut self) -> io::Result<()> {
        V3::write_magic(&mut self.inner)
    }
}

impl ItemWriter<io::BufWriter<fs::File>> {
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

impl<W> BgzfItemWriter<W>
where
    W: io::Write,
{
    /// Creates a new BGZF writer.
    pub fn from_bgzf(inner: W) -> Self {
        Self::new(bgzf::Writer::new(inner))
    }
}

impl BgzfItemWriter<io::BufWriter<fs::File>> {
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

impl<W> From<W> for ItemWriter<W>
where
    W: io::Write,
{
    fn from(inner: W) -> Self {
        Self::new(inner)
    }
}
