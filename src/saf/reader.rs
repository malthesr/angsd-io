//! Reading of the SAF format.

use std::{fs, io, marker::PhantomData, path::Path};

use crate::ReadStatus;

use super::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    index::Index,
    record::{Id, Likelihoods, Record},
    Version,
};

mod intersect;
pub use intersect::Intersect;

mod traits;
pub use traits::{ReadableInto, ReaderExt};

/// A BGZF SAF reader.
///
/// Note that this is a type alias for a [`Reader`], and most methods are
/// available via the [`Reader`] type.
pub type BgzfReader<R, V> = Reader<bgzf::Reader<R>, V>;

/// A SAF reader.
pub struct Reader<R, V> {
    index: Index,
    position_reader: R,
    item_reader: R,
    position: ReaderPosition,
    v: PhantomData<V>,
}

impl<R, V> Reader<R, V>
where
    R: io::BufRead,
    V: Version,
{
    /// Returns a new record suitable for use in reading.
    ///
    /// The [`Self::read_record`] method requires an input record buffer with the correct number of
    /// alleles. This method creates such a record, using the number of alleles defined in the index.
    pub fn create_record_buf(&self) -> Record<Id, Likelihoods> {
        Record::from_alleles(0, 1, self.index.alleles())
    }

    /// Returns the index.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Returns a mutable reference to the index.
    pub fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    /// Returns the inner index, position reader, and item reader, consuming `self`.
    pub fn into_parts(self) -> (Index, R, R) {
        (self.index, self.position_reader, self.item_reader)
    }

    /// Returns the inner item reader.
    pub fn item_reader(&self) -> &R {
        &self.item_reader
    }

    /// Returns a mutable reference to the inner item reader.
    pub fn item_reader_mut(&mut self) -> &mut R {
        &mut self.item_reader
    }

    /// Creates a new reader.
    ///
    /// # Returns
    ///
    /// `None` if `index` contains no records.
    pub fn new(index: Index, position_reader: R, item_reader: R) -> Option<Self> {
        let position = ReaderPosition::setup(&index)?;

        Some(Self {
            index,
            position_reader,
            item_reader,
            position,
            v: PhantomData,
        })
    }

    /// Returns the inner position reader.
    pub fn position_reader(&self) -> &R {
        &self.position_reader
    }

    /// Returns a mutable reference to the inner position reader.
    pub fn position_reader_mut(&mut self) -> &mut R {
        &mut self.position_reader
    }

    /// Reads a single item from the item reader into the provided buffer.
    fn read_item(&mut self, buf: &mut V::Item) -> io::Result<ReadStatus> {
        V::read_item(&mut self.item_reader, buf)
    }

    /// Reads and checks the magic numbers.
    ///
    /// Assumes the streams are positioned at the beginning of the files.
    pub fn read_magic(&mut self) -> io::Result<()> {
        V::read_magic(&mut self.position_reader).and_then(|_| V::read_magic(&mut self.item_reader))
    }

    /// Reads a single position from the item reader.
    fn read_position(&mut self) -> io::Result<Option<u32>> {
        self.position_reader.read_position()
    }

    /// Reads a single record.
    pub fn read_record(&mut self, record: &mut Record<Id, V::Item>) -> io::Result<ReadStatus> {
        if !self.position.contig_is_finished() || self.position.next_contig(&self.index).is_some() {
            // Index still contains data, read and check that readers are not at EoF
            match (self.read_position()?, self.read_item(record.item_mut())?) {
                (Some(pos), ReadStatus::NotDone) => {
                    *record.contig_id_mut() = self.position.contig_id();
                    *record.position_mut() = pos;

                    self.position.next_site_on_contig();

                    Ok(ReadStatus::NotDone)
                }
                (Some(_), ReadStatus::Done) => Err(eof_err(
                    "reached EoF in SAF position file before reaching EoF in SAF item file",
                )),
                (None, ReadStatus::NotDone) => Err(eof_err(
                    "reached EoF in SAF item file before reaching EoF in SAF position file",
                )),
                (None, ReadStatus::Done) => Err(eof_err(
                    "reached EoF in both SAF files before reaching end of index",
                )),
            }
        } else {
            // Reached end of index, check that readers are at EoF
            let position_reader_is_done = ReadStatus::check(&mut self.position_reader)?.is_done();
            let item_reader_is_done = ReadStatus::check(&mut self.item_reader)?.is_done();

            match (position_reader_is_done, item_reader_is_done) {
                (true, true) => Ok(ReadStatus::Done),
                (true, false) => Err(data_err(
                    "reached end of index before reaching EoF in SAF position file",
                )),
                (false, true) => Err(data_err(
                    "reached end of index before reaching EoF in SAF item file",
                )),
                (false, false) => Err(data_err(
                    "reached end of index before reaching EoF in both SAF files",
                )),
            }
        }
    }
}

impl<R, V> BgzfReader<R, V>
where
    R: io::BufRead + io::Seek,
    V: Version,
{
    /// Creates an intersection of two readers.
    ///
    /// The resulting intersecting readers will read only records that lie on the same contigs
    /// and the same positions. Further readers can be added to the resulting intersecting reader
    /// by chaining the [`Intersect::intersect`] method.
    pub fn intersect(self, other: Self) -> Intersect<R, V> {
        Intersect::from_reader(self).intersect(other)
    }

    /// Seeks to start of contig.
    ///
    /// The `contig_id` refers to the position of records in the index.
    ///
    /// # Panics
    ///
    /// Panics if `contig_id` is larger than the number of records defined in the index.
    pub fn seek(&mut self, contig_id: usize) -> io::Result<()> {
        self.position
            .set_contig(&self.index, contig_id)
            .expect("cannot seek to contig ID");

        let record = &self.index.records()[contig_id];

        let position_offset = bgzf::VirtualPosition::from(record.position_offset());
        self.position_reader.seek(position_offset)?;

        let item_offset = bgzf::VirtualPosition::from(record.item_offset());
        self.item_reader.seek(item_offset)?;

        Ok(())
    }

    /// Seeks to start of contig by name.
    ///
    /// Note that this requires a linear search of names in the index with worst time complexity
    /// linear in the index size.. If the index is large, and the contig ID is known, prefer
    /// [`Self::seek`] is more efficient.
    ///
    /// # Panics
    ///
    /// Panics if sequence name is not defined in index.
    pub fn seek_by_name(&mut self, name: &str) -> io::Result<()> {
        let contig_id = self
            .index
            .records()
            .iter()
            .position(|x| x.name() == name)
            .expect("name not found in index");

        self.seek(contig_id)
    }
}

impl<V> BgzfReader<io::BufReader<fs::File>, V>
where
    V: Version,
{
    /// Creates a new BGZF reader from any member path.
    ///
    /// This method relies on stripping a conventional suffix from the member path and
    /// reconstructing all member paths. See [`Self::from_bgzf_prefix`] for details on
    /// conventional naming.
    ///
    /// The stream will be positioned immediately after the magic number.
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

    /// Creates a new BGZF reader from paths.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_bgzf_paths<P>(index_path: P, position_path: P, item_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index = Index::read_from_path(index_path)?;
        let position_reader = open_bgzf(position_path)?;
        let item_reader = open_bgzf(item_path)?;

        let mut new = Self::new(index, position_reader, item_reader).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "SAF index contains no records")
        })?;

        new.read_magic()?;

        Ok(new)
    }

    /// Creates a new BGZF reader from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and item files are named according to a shared
    /// prefix and specific extensions for each file. See [`crate::saf::ext`] for these extensions.
    /// Where this convention is observed, this method opens a reader from the shared prefix.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_bgzf_prefix<P>(prefix: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let [index_path, position_path, item_path] =
            member_paths_from_prefix(&prefix.as_ref().to_string_lossy());

        Self::from_bgzf_paths(index_path, position_path, item_path)
    }
}

/// Creates a new BGZF reader from a path.
fn open_bgzf<P>(path: P) -> io::Result<bgzf::Reader<io::BufReader<fs::File>>>
where
    P: AsRef<Path>,
{
    fs::File::open(path)
        .map(io::BufReader::new)
        .map(bgzf::Reader::new)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ReaderPosition {
    contig_id: usize,
    sites: usize,
}

impl ReaderPosition {
    fn contig_id(&self) -> usize {
        self.contig_id
    }

    fn contig_is_finished(&self) -> bool {
        0 == self.sites
    }

    fn next_site_on_contig(&mut self) {
        self.sites -= 1
    }

    fn next_contig(&mut self, index: &Index) -> Option<()> {
        self.set_contig(index, self.contig_id + 1)
    }

    fn set_contig(&mut self, index: &Index, contig_id: usize) -> Option<()> {
        self.contig_id = contig_id;

        self.sites = index.records().get(self.contig_id)?.sites();

        Some(())
    }

    fn setup(index: &Index) -> Option<Self> {
        let contig_id = 0;
        let sites = index.records().first()?.sites();

        Some(Self { contig_id, sites })
    }
}

fn eof_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, msg)
}

fn data_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}
