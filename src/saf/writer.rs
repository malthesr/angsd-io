//! Writing of the SAF format.

use std::{fs, io, path::Path};

use super::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    index,
    record::Record,
    version::{Version, V3, V4},
};

mod traits;
pub(crate) use traits::WriterExt;

/// A SAF writer for the [`V3`] format.
pub type WriterV3<R> = Writer<R, V3>;

/// A SAF writer for the [`V4`] format.
pub type WriterV4<R> = Writer<R, V4>;

/// A SAF writer.
///
/// The writer is generic over the inner writer type and over the SAF [`Version`] being read.
/// Version-specific aliases [`WriterV3`] and [`WriterV4`] are provided for convenience.
pub struct Writer<W, V>
where
    W: io::Write,
{
    pub(super) index_writer: W,
    pub(super) position_writer: bgzf::Writer<W>,
    pub(super) item_writer: bgzf::Writer<W>,
    pub(super) index_record: Option<index::Record<V>>,
}

impl<W, V> Writer<W, V>
where
    W: io::Write,
    V: Version,
{
    /// Finishes writing.
    pub fn finish(mut self) -> io::Result<(W, W, W)> {
        if let Some(record) = self.index_record {
            record.write(&mut self.index_writer)?;
        }

        Ok((
            self.index_writer,
            self.position_writer.finish()?,
            self.item_writer.finish()?,
        ))
    }

    /// Creates a new writer from existing BGZF writers.
    pub fn from_bgzf(
        index_writer: W,
        position_writer: bgzf::Writer<W>,
        item_writer: bgzf::Writer<W>,
    ) -> Self {
        Self {
            index_writer,
            position_writer,
            item_writer,
            index_record: None,
        }
    }

    /// Returns the index writer.
    pub fn index_writer(&self) -> &W {
        &self.index_writer
    }

    /// Returns a mutable reference to the index.
    pub fn index_writer_mut(&mut self) -> &mut W {
        &mut self.index_writer
    }

    /// Returns the inner index, position writer, and item writer, consuming `self`.
    pub fn into_parts(self) -> (W, bgzf::Writer<W>, bgzf::Writer<W>) {
        (self.index_writer, self.position_writer, self.item_writer)
    }

    /// Returns the inner item writer.
    pub fn item_writer(&self) -> &bgzf::Writer<W> {
        &self.item_writer
    }

    /// Returns a mutable reference to the inner item writer.
    pub fn item_writer_mut(&mut self) -> &mut bgzf::Writer<W> {
        &mut self.item_writer
    }

    /// Creates a new writer.
    ///
    /// The provided writers will be wrapped in [`bgzf::Writer`]s. To create a writer from existing
    /// BGZF writers, see [`Self::from_bgzf`].
    pub fn new(index_writer: W, position_writer: W, item_writer: W) -> Self {
        Self::from_bgzf(
            index_writer,
            bgzf::Writer::new(position_writer),
            bgzf::Writer::new(item_writer),
        )
    }

    /// Returns the inner position writer.
    pub fn position_writer(&self) -> &bgzf::Writer<W> {
        &self.position_writer
    }

    /// Returns a mutable reference to the inner position writer.
    pub fn position_writer_mut(&mut self) -> &mut bgzf::Writer<W> {
        &mut self.position_writer
    }

    /// Writes the number alleles to the index writer.
    ///
    /// The number of alleles should be written immediately after the magic number.
    pub fn write_alleles(&mut self, alleles: usize) -> io::Result<()> {
        self.index_writer.write_all(&alleles.to_le_bytes())
    }

    /// Writes the magic numbers.
    ///
    /// The magic numbers should be written as the first thing.
    pub fn write_magic(&mut self) -> io::Result<()> {
        V::write_magic(&mut self.index_writer)
            .and_then(|_| V::write_magic(&mut self.position_writer))
            .and_then(|_| V::write_magic(&mut self.item_writer))
    }

    /// Writes a single record.
    pub fn write_record<I>(&mut self, record: &Record<I, V::Item>) -> io::Result<()>
    where
        I: AsRef<str>,
    {
        V::write_record(self, record)
    }
}

impl<V> Writer<io::BufWriter<fs::File>, V>
where
    V: Version,
{
    /// Creates a new writer from any member path.
    ///
    /// This method relies on stripping a conventional suffix from the member path and
    /// reconstructing all member paths. See [`Self::from_prefix`] for details on
    /// conventional naming.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths, and the alleles will be written to the index
    /// writer after the magic number.
    pub fn from_member_path<P>(alleles: usize, member_path: P) -> io::Result<Self>
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

        Self::from_prefix(alleles, prefix)
    }

    /// Creates a new writer from paths.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths, and the alleles will be written to the index
    /// writer after the magic number.
    pub fn from_paths<P>(
        alleles: usize,
        index_path: P,
        position_path: P,
        item_path: P,
    ) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index_writer = fs::File::create(index_path).map(io::BufWriter::new)?;
        let position_writer = fs::File::create(position_path).map(io::BufWriter::new)?;
        let item_writer = fs::File::create(item_path).map(io::BufWriter::new)?;

        let mut new = Self::new(index_writer, position_writer, item_writer);
        new.write_magic()?;
        new.write_alleles(alleles)?;
        Ok(new)
    }

    /// Creates a new writer from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and item files are named according to a shared
    /// prefix and specific extensions for each file. See [`crate::saf::ext`] for these extensions.
    /// This method opens files for writing in accordance with these conventions.
    ///
    /// If the paths already exists, they will be overwritten.
    ///
    /// The magic number will be written to the paths, and the alleles will be written to the index
    /// writer after the magic number.
    pub fn from_prefix<P>(alleles: usize, prefix: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let [index_path, position_path, item_path] =
            member_paths_from_prefix(&prefix.as_ref().to_string_lossy());

        Self::from_paths(alleles, index_path, position_path, item_path)
    }
}
