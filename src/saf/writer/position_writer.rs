use std::{fs, io, path::Path};

use byteorder::WriteBytesExt;

use crate::saf::{write_magic, Endian};

/// A BGZF SAF position writer.
///
/// Note that this is a type alias for a [`PositionWriter`], and most methods are
/// available via the [`PositionWriter`] type.
pub type BgzfPositionWriter<W> = PositionWriter<bgzf::Writer<W>>;

/// A SAF position writer.
pub struct PositionWriter<W> {
    inner: W,
}

impl<W> PositionWriter<W>
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

    /// Writes a single position.
    pub fn write_position(&mut self, position: u32) -> io::Result<()> {
        self.inner.write_u32::<Endian>(position)
    }

    /// Writes the magic number.
    pub fn write_magic(&mut self) -> io::Result<()> {
        write_magic(&mut self.inner)
    }
}

impl PositionWriter<io::BufWriter<fs::File>> {
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

impl<W> BgzfPositionWriter<W>
where
    W: io::Write,
{
    /// Creates a new BGZF writer.
    pub fn from_bgzf(inner: W) -> Self {
        Self::new(bgzf::Writer::new(inner))
    }
}

impl BgzfPositionWriter<io::BufWriter<fs::File>> {
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

impl<W> From<W> for PositionWriter<W>
where
    W: io::Write,
{
    fn from(inner: W) -> Self {
        Self::new(inner)
    }
}
