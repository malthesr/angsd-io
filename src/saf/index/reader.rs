use std::{fs, io, marker::PhantomData, mem, path::Path, string};

use crate::saf::{Version, V3};

use super::{Index, Record};

/// A SAF index reader.
pub struct Reader<R, V: Version = V3> {
    inner: R,
    v: PhantomData<V>,
}

impl<R, V> Reader<R, V>
where
    R: io::BufRead,
    V: Version,
{
    /// Returns the inner reader.
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
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            v: PhantomData,
        }
    }

    /// Reads an entire index.
    ///
    /// The stream is assumed to be positioned at the beginning of the file.
    ///
    /// See also the [`Index::read_from_path`] convenience method.
    pub fn read_index(&mut self) -> io::Result<Index<V3>> {
        self.read_magic()?;

        let alleles = self.read_alleles()?;

        let mut records = Vec::new();
        while self.has_data_left()? {
            let record = self.read_record()?;

            records.push(record)
        }

        Ok(Index::new(alleles, records))
    }

    fn has_data_left(&mut self) -> io::Result<bool> {
        self.inner.fill_buf().map(|buf| !buf.is_empty())
    }

    fn read_alleles(&mut self) -> io::Result<usize> {
        let mut buf = [0; mem::size_of::<usize>()];
        self.inner.read_exact(&mut buf)?;

        Ok(usize::from_le_bytes(buf))
    }

    fn read_record(&mut self) -> io::Result<Record> {
        let mut usize_buf = [0; mem::size_of::<usize>()];
        self.inner.read_exact(&mut usize_buf)?;
        let name_len = usize::from_le_bytes(usize_buf);

        let mut name_buf = vec![0; name_len];
        self.inner.read_exact(&mut name_buf)?;
        let name = string::String::from_utf8(name_buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "index record name not valid UTF8",
            )
        })?;

        self.inner.read_exact(&mut usize_buf)?;
        let sites = usize::from_le_bytes(usize_buf);

        let mut offset_buf = [0; 8];
        self.inner.read_exact(&mut offset_buf)?;
        let position_offset = u64::from_le_bytes(offset_buf);

        self.inner.read_exact(&mut offset_buf)?;
        let item_offset = u64::from_le_bytes(offset_buf);

        Ok(Record::new(name, sites, position_offset, item_offset))
    }

    fn read_magic(&mut self) -> io::Result<()> {
        V::read_magic(&mut self.inner)
    }
}

impl<V> Reader<io::BufReader<fs::File>, V>
where
    V: Version,
{
    /// Creates a new reader from a path.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::open(path).map(io::BufReader::new).map(Self::new)
    }
}

impl<R, V> From<R> for Reader<R, V>
where
    R: io::BufRead,
    V: Version,
{
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}
