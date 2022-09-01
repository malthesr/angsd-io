//! Writing of the SAF format.

use std::{fs, io, mem, path::Path};

use super::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    index::{self, IndexWriterExt},
    record::{Likelihoods, Record},
    Version, V3,
};

mod item_writer;
pub use item_writer::{BgzfItemWriter, ItemWriter};

mod traits;
pub use traits::WriterExt;

const START_OFFSET: u64 = 8;

/// A BGZF SAF writer.
///
/// Note that this is a type alias for a [`Writer`], and most methods are
/// available via the [`Writer`] type.
pub type BgzfWriter<W1, W2, V> = Writer<W1, bgzf::Writer<W2>, V>;

/// A SAF writer.
pub struct Writer<W1, W2, V: Version = V3> {
    index_writer: W1,
    position_writer: W2,
    item_writer: ItemWriter<W2>,
    index_record: Option<index::Record<V>>,
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
    pub fn into_parts(self) -> (W1, W2, ItemWriter<W2>) {
        (self.index_writer, self.position_writer, self.item_writer)
    }

    /// Returns the inner item writer.
    pub fn item_writer(&self) -> &ItemWriter<W2> {
        &self.item_writer
    }

    /// Returns a mutable reference to the inner item writer.
    pub fn item_writer_mut(&mut self) -> &mut ItemWriter<W2> {
        &mut self.item_writer
    }

    /// Creates a new writer.
    pub fn new(index_writer: W1, position_writer: W2, item_writer: ItemWriter<W2>) -> Self {
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
        V::write_magic(&mut self.position_writer).and_then(|_| self.item_writer.write_magic())
    }
}

impl<W1, W2> BgzfWriter<W1, W2, V3>
where
    W1: io::Write,
    W2: io::Write,
{
    /// Writes a single record.
    pub fn write_record<I>(&mut self, record: &Record<I, Likelihoods>) -> io::Result<()>
    where
        I: AsRef<str>,
    {
        let contig_id = record.contig_id().as_ref();

        // Handle index according to three cases:
        //
        // (1) New record is not the first, and...
        //     (1a) it is on a new contig: write the current index record and setup next
        //     (1b) is on the old contig: increment the count of sites on contig
        // (2) New record is the first: write alleles to index, and set up first index record
        if let Some(index_record) = self.index_record.as_mut() {
            // Case (1)
            if index_record.name() != contig_id {
                // Case (1a)
                let position_offset = u64::from(self.position_writer.virtual_position());
                let item_offset = u64::from(self.item_writer.get_ref().virtual_position());

                let old = mem::replace(
                    index_record,
                    index::Record::new(contig_id.to_string(), 1, position_offset, item_offset),
                );

                old.write(&mut self.index_writer)?;
            } else {
                // Case (1b)
                *index_record.sites_mut() += 1;
            }
        } else {
            // Case (2)
            self.index_writer.write_alleles(record.alleles())?;

            self.index_record = Some(index::Record::new(
                contig_id.to_string(),
                1,
                START_OFFSET,
                START_OFFSET,
            ));
        }

        // Write record
        self.position_writer.write_position(record.position())?;
        self.item_writer.write_item(record.item())?;

        Ok(())
    }

    /// Finishes writing.
    pub fn finish(mut self) -> io::Result<(W1, W2, W2)> {
        if let Some(record) = self.index_record {
            record.write(&mut self.index_writer)?;
        }

        Ok((
            self.index_writer,
            self.position_writer.finish()?,
            self.item_writer.into_inner().finish()?,
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
        let mut index_writer = fs::File::create(index_path).map(io::BufWriter::new)?;
        V::write_magic(&mut index_writer)?;

        let mut position_writer = open_bgzf(position_path)?;
        V::write_magic(&mut position_writer)?;

        let item_writer = ItemWriter::from_bgzf_path(item_path)?;

        Ok(Self::new(index_writer, position_writer, item_writer))
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
