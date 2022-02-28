use std::{fs, io, path::Path};

use byteorder::ReadBytesExt;

use crate::{
    saf::{read_magic, Endian},
    ReadStatus,
};

/// A BGZF SAF value reader.
///
/// Note that this is a type alias for a [`ValueReader`], and most methods are
/// available via the [`ValueReader`] type.
pub type BgzfValueReader<R> = ValueReader<bgzf::Reader<R>>;

/// A SAF value reader.
pub struct ValueReader<R> {
    inner: R,
}

impl<R> ValueReader<R>
where
    R: io::BufRead,
{
    /// Returns a mutable reference to the inner reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the inner reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns the inner reader, consuming `self.`
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Creates a new reader.
    ///
    /// Note that the constructed reader will not be a BGZF reader unless `R` is
    /// a BGZF reader. To construct a BGZF reader, see the
    /// [`BgzfValueReader::from_bgzf`] constructor.
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads and checks the magic number.
    ///
    /// Assumes the stream is positioned at the beginning of the file.
    pub fn read_magic(&mut self) -> io::Result<()> {
        read_magic(&mut self.inner)
    }

    /// Reads a single value.
    pub fn read_value(&mut self) -> io::Result<f32> {
        self.inner.read_f32::<Endian>()
    }

    /// Reads multiple values.
    pub fn read_values(&mut self, values: &mut [f32]) -> io::Result<ReadStatus> {
        if ReadStatus::check(&mut self.inner)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        self.inner
            .read_f32_into::<Endian>(values)
            .map(|_| ReadStatus::NotDone)
    }
}

impl ValueReader<io::BufReader<fs::File>> {
    /// Creates a new reader from a path.
    ///
    /// Note that the constructed reader will not be a BGZF reader.
    /// To construct a BGZF reader from path, see the
    /// [`BgzfValueReader::from_bgzf_path`] constructor.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut reader = fs::File::open(path)
            .map(io::BufReader::new)
            .map(Self::new)?;

        reader.read_magic()?;

        Ok(reader)
    }
}

impl<R> BgzfValueReader<R>
where
    R: io::BufRead,
{
    /// Creates a new BGZF reader.
    ///
    /// This will wrap the inner reader `R` in a BGZF reader, so `R` should
    /// *not* be a BGZF reader.
    pub fn from_bgzf(inner: R) -> Self {
        Self::new(bgzf::Reader::new(inner))
    }
}

impl BgzfValueReader<io::BufReader<fs::File>> {
    /// Creates a new BGZF reader from a path.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_bgzf_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut reader = fs::File::open(path)
            .map(io::BufReader::new)
            .map(Self::from_bgzf)?;

        reader.read_magic()?;

        Ok(reader)
    }
}

impl<R> From<R> for ValueReader<R>
where
    R: io::BufRead,
{
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}
