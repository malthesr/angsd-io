//! Reading of the SAF format.

use std::io;

use crate::ReadStatus;

use super::{
    index::Index,
    record::{Id, Record},
    version::{Version, V3, V4},
};

mod builder;
pub use builder::Builder;

mod intersect;
pub use intersect::Intersect;

mod traits;
pub(crate) use traits::ReaderExt;

/// A SAF reader for the [`V3`] format.
pub type ReaderV3<R> = Reader<R, V3>;

/// A SAF reader for the [`V4`] format.
pub type ReaderV4<R> = Reader<R, V4>;

/// A SAF reader.
///
/// The reader is generic over the inner reader type and over the SAF [`Version`] being read.
/// Version-specific aliases [`ReaderV3`] and [`ReaderV4`] are provided for convenience.
pub struct Reader<R, V> {
    location: Location<V>,
    position_reader: bgzf::Reader<R>,
    item_reader: bgzf::Reader<R>,
}

impl<R, V> Reader<R, V>
where
    R: io::BufRead,
    V: Version,
{
    /// Returns a new record suitable for use in reading.
    pub fn create_record_buf(&self) -> Record<Id, V::Item> {
        V::create_record_buf(self.index())
    }

    /// Creates a new reader from its raw parts.
    ///
    /// A [`Builder`] will typically be a more ergonimic way to create a reader.
    ///
    /// Returns [`None`] if index contains no records.
    pub fn from_bgzf(
        index: Index<V>,
        position_reader: bgzf::Reader<R>,
        item_reader: bgzf::Reader<R>,
    ) -> Option<Self> {
        Location::setup(index).map(|location| Self {
            location,
            position_reader,
            item_reader,
        })
    }

    /// Returns the index.
    pub fn index(&self) -> &Index<V> {
        &self.location.index
    }

    /// Returns a mutable reference to the index.
    pub fn index_mut(&mut self) -> &mut Index<V> {
        &mut self.location.index
    }

    /// Returns the inner index, position reader, and item reader, consuming `self`.
    pub fn into_parts(self) -> (Index<V>, bgzf::Reader<R>, bgzf::Reader<R>) {
        (self.location.index, self.position_reader, self.item_reader)
    }

    /// Returns the inner item reader.
    pub fn item_reader(&self) -> &bgzf::Reader<R> {
        &self.item_reader
    }

    /// Returns a mutable reference to the inner item reader.
    pub fn item_reader_mut(&mut self) -> &mut bgzf::Reader<R> {
        &mut self.item_reader
    }

    /// Returns the inner position reader.
    pub fn position_reader(&self) -> &bgzf::Reader<R> {
        &self.position_reader
    }

    /// Returns a mutable reference to the inner position reader.
    pub fn position_reader_mut(&mut self) -> &mut bgzf::Reader<R> {
        &mut self.position_reader
    }

    /// Reads a single item from the item reader into the provided buffer.
    ///
    /// Note that this will bring the item and position readers out of sync. Use
    /// [`Self::read_record`] instead unless you wish to manually re-sync the underlying readers.
    pub fn read_item(&mut self, buf: &mut V::Item) -> io::Result<ReadStatus> {
        V::read_item(&mut self.item_reader, buf)
    }

    /// Reads and checks the magic numbers.
    ///
    /// Assumes the streams are positioned at the beginning of the files.
    pub fn read_magic(&mut self) -> io::Result<()> {
        V::read_magic(&mut self.position_reader).and_then(|_| V::read_magic(&mut self.item_reader))
    }

    /// Reads a single position from the position reader.
    ///
    /// Note that this will bring the item and position readers out of sync. Use
    /// [`Self::read_record`] instead unless you wish to manually re-sync the underlying readers.
    pub fn read_position(&mut self) -> io::Result<Option<u32>> {
        self.position_reader.read_position()
    }

    /// Reads a single record.
    ///
    /// Note that the record buffer needs to be correctly set up. Use [`Self::create_record_buf`]
    /// for a correctly initialised record buffer to use for reading.
    pub fn read_record(&mut self, record: &mut Record<Id, V::Item>) -> io::Result<ReadStatus> {
        if !self.location.contig_is_finished() || self.location.next_contig().is_some() {
            // Index still contains data, read and check that readers are not at EoF
            match (self.read_position()?, self.read_item(record.item_mut())?) {
                (Some(pos), ReadStatus::NotDone) => {
                    *record.contig_id_mut() = self.location.contig_id;
                    *record.position_mut() = pos;

                    self.location.next_site_on_contig();

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

impl<R, V> Reader<R, V>
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
        self.location
            .set_contig(contig_id)
            .expect("cannot seek to contig ID");

        let record = &self.index().records()[contig_id];
        let position_offset = record.position_offset();
        let item_offset = record.item_offset();

        let position_vpos = bgzf::VirtualPosition::from(position_offset);
        self.position_reader.seek(position_vpos)?;

        let item_vpos = bgzf::VirtualPosition::from(item_offset);
        self.item_reader.seek(item_vpos)?;

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
            .index()
            .records()
            .iter()
            .position(|x| x.name() == name)
            .expect("name not found in index");

        self.seek(contig_id)
    }
}

/// A SAF reader location.
///
/// The location tracks the current location of the reader relative to its index file in terms
/// of which contig is currently being read, and how many sites are left on that contig.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Location<V> {
    pub index: Index<V>,
    pub contig_id: usize,
    pub sites_left_on_contig: usize,
}

impl<V> Location<V>
where
    V: Version,
{
    /// Returns `true` if no more sites are left to read on the current contig.
    pub fn contig_is_finished(&self) -> bool {
        0 == self.sites_left_on_contig
    }

    /// Decrements the number of sites left to read on current contig.
    pub fn next_site_on_contig(&mut self) {
        self.sites_left_on_contig -= 1
    }

    /// Moves the location first site on the next contig in index.
    ///
    /// Returns `None` is no more contigs exist in the index.
    pub fn next_contig(&mut self) -> Option<()> {
        self.set_contig(self.contig_id + 1)
    }

    /// Moves the location to the first site on the contig with the provided ID in the index.
    ///
    /// Returns `None` if contig with the provided ID does not exist in the index.
    pub fn set_contig(&mut self, contig_id: usize) -> Option<()> {
        self.contig_id = contig_id;
        self.sites_left_on_contig = self.index.records().get(self.contig_id)?.sites();
        Some(())
    }

    /// Creates a new location from an index.
    ///
    /// The location will be set to the first site on the first contig. Returns `None` if no contigs
    /// are defined in the index.
    pub fn setup(index: Index<V>) -> Option<Self> {
        let contig_id = 0;
        let sites_left_on_contig = index.records().first()?.sites();

        Some(Self {
            index,
            contig_id,
            sites_left_on_contig,
        })
    }
}

fn eof_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, msg)
}

fn data_err(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}
