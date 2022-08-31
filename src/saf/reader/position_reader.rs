use std::{fs, io, path::Path};

use crate::saf::{Version, V3};

const BYTES: usize = std::mem::size_of::<u32>();

/// A BGZF SAF position reader.
///
/// Note that this is a type alias for a [`PositionReader`], and most methods are
/// available via the [`PositionReader`] type.
pub type BgzfPositionReader<R> = PositionReader<bgzf::Reader<R>>;

/// A SAF position reader.
pub struct PositionReader<R> {
    inner: R,
    buf: [u8; BYTES],
}

impl<R> PositionReader<R>
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
    /// [`BgzfPositionReader::from_bgzf`] constructor.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buf: [0; BYTES],
        }
    }

    /// Reads a single position.
    ///
    /// # Returns
    ///
    /// `None` if reader is at EoF.
    pub fn read_position(&mut self) -> io::Result<Option<u32>> {
        // Modified from std::io::default_read_exact
        let mut buf = self.buf.as_mut_slice();

        while !buf.is_empty() {
            match self.inner.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        if buf.len() == 4 {
            Ok(None)
        } else if !buf.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to read position",
            ))
        } else {
            Ok(Some(u32::from_le_bytes(self.buf)))
        }
    }
}

impl PositionReader<io::BufReader<fs::File>> {
    /// Creates a new reader from a path.
    ///
    /// Note that the constructed reader will not be a BGZF reader.
    /// To construct a BGZF reader from path, see the
    /// [`BgzfPositionReader::from_bgzf_path`] constructor.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut reader = fs::File::open(path)
            .map(io::BufReader::new)
            .map(Self::new)?;

        V3::read_magic(&mut reader.inner)?;

        Ok(reader)
    }
}

impl<R> BgzfPositionReader<R>
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

impl BgzfPositionReader<io::BufReader<fs::File>> {
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

        V3::read_magic(&mut reader.inner)?;

        Ok(reader)
    }
}

impl<R> From<R> for PositionReader<R>
where
    R: io::BufRead,
{
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}
