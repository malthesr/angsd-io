//! Writing of the SAF format.

use std::{fs, io, path::Path};

use super::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    index,
    record::Record,
    Version,
};

mod traits;
pub(crate) use traits::WriterExt;

/// A BGZF SAF writer.
///
/// Note that this is a type alias for a [`Writer`], and most methods are
/// available via the [`Writer`] type.
pub type BgzfWriter<W1, W2, V> = Writer<W1, bgzf::Writer<W2>, V>;

/// A SAF writer.
pub struct Writer<W1, W2, V> {
    pub(super) index_writer: W1,
    pub(super) position_writer: W2,
    pub(super) item_writer: W2,
    pub(super) index_record: Option<index::Record<V>>,
}

impl<W1, W2, V> Writer<W1, W2, V>
where
    W1: io::Write,
    W2: io::Write,
    V: Version,
{
    /// Returns the index writer.
    pub fn index_writer(&self) -> &W1 {
        &self.index_writer
    }

    /// Returns a mutable reference to the index.
    pub fn index_writer_mut(&mut self) -> &mut W1 {
        &mut self.index_writer
    }

    /// Returns the inner index, position writer, and item writer, consuming `self`.
    pub fn into_parts(self) -> (W1, W2, W2) {
        (self.index_writer, self.position_writer, self.item_writer)
    }

    /// Returns the inner item writer.
    pub fn item_writer(&self) -> &W2 {
        &self.item_writer
    }

    /// Returns a mutable reference to the inner item writer.
    pub fn item_writer_mut(&mut self) -> &mut W2 {
        &mut self.item_writer
    }

    /// Creates a new writer.
    pub fn new(index_writer: W1, position_writer: W2, item_writer: W2) -> Self {
        Self {
            index_writer,
            position_writer,
            item_writer,
            index_record: None,
        }
    }

    /// Returns the inner position writer.
    pub fn position_writer(&self) -> &W2 {
        &self.position_writer
    }

    /// Returns a mutable reference to the inner position writer.
    pub fn position_writer_mut(&mut self) -> &mut W2 {
        &mut self.position_writer
    }

    /// Writes the magic numbers.
    pub fn write_magic(&mut self) -> io::Result<()> {
        V::write_magic(&mut self.index_writer)
            .and_then(|_| V::write_magic(&mut self.position_writer))
            .and_then(|_| V::write_magic(&mut self.item_writer))
    }
}

impl<W1, W2, V> BgzfWriter<W1, W2, V>
where
    W1: io::Write,
    W2: io::Write,
    V: Version,
{
    /// Writes a single record.
    pub fn write_record<I>(&mut self, record: &Record<I, V::Item>) -> io::Result<()>
    where
        I: AsRef<str>,
    {
        V::write_record(self, record)
    }

    /// Finishes writing.
    pub fn finish(mut self) -> io::Result<(W1, W2, W2)> {
        if let Some(record) = self.index_record {
            record.write(&mut self.index_writer)?;
        }

        Ok((
            self.index_writer,
            self.position_writer.finish()?,
            self.item_writer.finish()?,
        ))
    }
}

impl<V> BgzfWriter<io::BufWriter<fs::File>, io::BufWriter<fs::File>, V>
where
    V: Version,
{
    /// Creates a new BGZF writer from any member path.
    ///
    /// This method relies on stripping a conventional suffix from the member path and
    /// reconstructing all member paths. See [`Self::from_bgzf_prefix`] for details on
    /// conventional naming.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths.
    pub fn from_bgzf_member_path<P>(member_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let s = member_path.as_ref().to_string_lossy();

        let prefix = prefix_from_member_path(&s).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Cannot determine shared SAF prefix from member path '{:?}'",
                    member_path.as_ref()
                ),
            )
        })?;

        Self::from_bgzf_prefix(prefix)
    }

    /// Creates a new BGZF writer from paths.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths.
    pub fn from_bgzf_paths<P>(index_path: P, position_path: P, item_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index_writer = fs::File::create(index_path).map(io::BufWriter::new)?;
        let position_writer = open_bgzf(position_path)?;
        let item_writer = open_bgzf(item_path)?;

        let mut new = Self::new(index_writer, position_writer, item_writer);

        new.write_magic()?;

        Ok(new)
    }

    /// Creates a new BGZF writer from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and item files are named according to a shared
    /// prefix and specific extensions for each file. See [`crate::saf::ext`] for these extensions.
    /// This method opens files for writing in accordance with these conventions.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths.
    pub fn from_bgzf_prefix<P>(prefix: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let [index_path, position_path, item_path] =
            member_paths_from_prefix(&prefix.as_ref().to_string_lossy());

        Self::from_bgzf_paths(index_path, position_path, item_path)
    }
}

/// Creates a new BGZF writer from a path.
fn open_bgzf<P>(path: P) -> io::Result<bgzf::Writer<io::BufWriter<fs::File>>>
where
    P: AsRef<Path>,
{
    fs::File::create(path)
        .map(io::BufWriter::new)
        .map(bgzf::Writer::new)
}
