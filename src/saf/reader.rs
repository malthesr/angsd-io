//! Reading of the SAF format.

use std::{fs, io, path::Path};

use crate::ReadStatus;

use super::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    index::Index,
    IdRecord,
};

mod intersect;
pub use intersect::Intersect;

mod position_reader;
pub use position_reader::{BgzfPositionReader, PositionReader};

mod value_reader;
pub use value_reader::{BgzfValueReader, ValueReader};

/// A BGZF SAF reader.
///
/// Note that this is a type alias for a [`Reader`], and most methods are
/// available via the [`Reader`] type.
pub type BgzfReader<R> = Reader<bgzf::Reader<R>>;

/// A SAF reader.
pub struct Reader<R> {
    index: Index,
    position_reader: PositionReader<R>,
    value_reader: ValueReader<R>,
    position: ReaderPosition,
}

impl<R> Reader<R>
where
    R: io::BufRead,
{
    /// Returns a new record suitable for use in reading.
    ///
    /// The [`Self::read_record`] method requires an input record buffer with the correct number of
    /// alleles. This method creates such a record, using the number of alleles defined in the index.
    pub fn create_record_buf(&self) -> IdRecord {
        IdRecord::from_alleles(0, 1, self.index.alleles())
    }

    /// Returns the index.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Returns a mutable reference to the index.
    pub fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }

    /// Returns the inner index, position reader, and value reader, consuming `self`.
    pub fn into_parts(self) -> (Index, PositionReader<R>, ValueReader<R>) {
        (self.index, self.position_reader, self.value_reader)
    }

    /// Creates a new reader.
    ///
    /// # Returns
    ///
    /// `None` if `index` contains no records.
    pub fn new(
        index: Index,
        position_reader: PositionReader<R>,
        value_reader: ValueReader<R>,
    ) -> Option<Self> {
        let position = ReaderPosition::setup(&index)?;

        Some(Self {
            index,
            position_reader,
            value_reader,
            position,
        })
    }

    /// Returns the inner position reader.
    pub fn position_reader(&self) -> &PositionReader<R> {
        &self.position_reader
    }

    /// Returns a mutable reference to the inner position reader.
    pub fn position_reader_mut(&mut self) -> &mut PositionReader<R> {
        &mut self.position_reader
    }

    /// Returns the inner value reader.
    pub fn value_reader(&self) -> &ValueReader<R> {
        &self.value_reader
    }

    /// Returns a mutable reference to the inner value reader.
    pub fn value_reader_mut(&mut self) -> &mut ValueReader<R> {
        &mut self.value_reader
    }

    /// Reads and checks the magic numbers.
    ///
    /// Assumes the streams are positioned at the beginning of the files.
    pub fn read_magic(&mut self) -> io::Result<()> {
        self.position_reader
            .read_magic()
            .and_then(|_| self.value_reader.read_magic())
    }

    /// Reads a single record.
    ///
    /// Note that the `record` must have a number of values defined in accordance with the number
    /// of values in the SAF values file. See [`Self::create_record_buf`] to create such a record
    /// based on the provided index.
    pub fn read_record(&mut self, record: &mut IdRecord) -> io::Result<ReadStatus> {
        if !self.position.contig_is_finished() || self.position.next_contig(&self.index).is_some() {
            // Index still contains data, read and check that readers are not at EoF
            match (
                self.position_reader.read_position()?,
                self.value_reader.read_values(record.values_mut())?,
            ) {
                (Some(pos), ReadStatus::NotDone) => {
                    *record.contig_id_mut() = self.position.contig_id();
                    *record.position_mut() = pos;

                    self.position.next_site_on_contig();

                    Ok(ReadStatus::NotDone)
                }
                (Some(_), ReadStatus::Done) => Err(eof_err(
                    "reached EoF in SAF position file before reaching EoF in SAF value file",
                )),
                (None, ReadStatus::NotDone) => Err(eof_err(
                    "reached EoF in SAF value file before reaching EoF in SAF position file",
                )),
                (None, ReadStatus::Done) => Err(eof_err(
                    "reached EoF in both SAF files before reaching end of index",
                )),
            }
        } else {
            // Reached end of index, check that readers are at EoF
            let position_reader_is_done =
                ReadStatus::check(self.position_reader.get_mut())?.is_done();
            let value_reader_is_done = ReadStatus::check(self.value_reader.get_mut())?.is_done();

            match (position_reader_is_done, value_reader_is_done) {
                (true, true) => Ok(ReadStatus::Done),
                (true, false) => Err(data_err(
                    "reached end of index before reaching EoF in SAF position file",
                )),
                (false, true) => Err(data_err(
                    "reached end of index before reaching EoF in SAF value file",
                )),
                (false, false) => Err(data_err(
                    "reached end of index before reaching EoF in both SAF files",
                )),
            }
        }
    }
}

impl<R> BgzfReader<R>
where
    R: io::BufRead + io::Seek,
{
    /// Creates an intersection of two readers.
    ///
    /// The resulting intersecting readers will read only records that lie on the same contigs
    /// and the same positions. Further readers can be added to the resulting intersecting reader
    /// by chaining the [`Intersect::intersect`] method.
    pub fn intersect(self, other: Self) -> Intersect<R> {
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
        self.position_reader.get_mut().seek(position_offset)?;

        let value_offset = bgzf::VirtualPosition::from(record.value_offset());
        self.value_reader.get_mut().seek(value_offset)?;

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

impl Reader<io::BufReader<fs::File>> {
    /// Creates a new reader from paths.
    ///
    /// Note that the constructed reader will not be a BGZF reader.
    /// To construct a BGZF reader from paths, see the
    /// [`BgzfReader::from_bgzf_paths`] constructor.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_paths<P>(index_path: P, position_path: P, value_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index = Index::read_from_path(index_path)?;
        let position_reader = PositionReader::from_path(position_path)?;
        let value_reader = ValueReader::from_path(value_path)?;

        Self::new(index, position_reader, value_reader).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "SAF index contains no records")
        })
    }
}

impl BgzfReader<io::BufReader<fs::File>> {
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
    pub fn from_bgzf_paths<P>(index_path: P, position_path: P, value_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index = Index::read_from_path(index_path)?;
        let position_reader = PositionReader::from_bgzf_path(position_path)?;
        let value_reader = ValueReader::from_bgzf_path(value_path)?;

        Self::new(index, position_reader, value_reader).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "SAF index contains no records")
        })
    }

    /// Creates a new BGZF reader from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and value files are named according to a shared
    /// prefix and specific extensions for each file. See [`crate::saf::ext`] for these extensions.
    /// Where this convention is observed, this method opens a reader from the shared prefix.
    ///
    /// The stream will be positioned immediately after the magic number.
    pub fn from_bgzf_prefix<P>(prefix: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let [index_path, position_path, value_path] =
            member_paths_from_prefix(&prefix.as_ref().to_string_lossy());

        Self::from_bgzf_paths(index_path, position_path, value_path)
    }
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
