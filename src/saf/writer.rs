//! Writing of the SAF format.

use std::{fs, io, mem, path::Path};

use super::{ext::*, index, Record};

mod position_writer;
pub use position_writer::{BgzfPositionWriter, PositionWriter};

mod value_writer;
pub use value_writer::{BgzfValueWriter, ValueWriter};

const START_OFFSET: u64 = super::MAGIC_NUMBER.len() as u64;

/// A BGZF SAF writer.
///
/// Note that this is a type alias for a [`Writer`], and most methods are
/// available via the [`Writer`] type.
pub type BgzfWriter<V, W> = Writer<V, bgzf::Writer<W>>;

/// A SAF writer.
pub struct Writer<V, W> {
    index_writer: index::Writer<V>,
    position_writer: PositionWriter<W>,
    value_writer: ValueWriter<W>,
    index_record: Option<index::Record>,
}

impl<V, W> Writer<V, W>
where
    V: io::Write,
    W: io::Write,
{
    /// Returns the index writer.
    pub fn index_writer(&self) -> &index::Writer<V> {
        &self.index_writer
    }

    /// Returns a mutable reference to the index.
    pub fn index_writer_mut(&mut self) -> &mut index::Writer<V> {
        &mut self.index_writer
    }

    /// Returns the inner index, position writer, and value writer, consuming `self`.
    pub fn into_parts(self) -> (index::Writer<V>, PositionWriter<W>, ValueWriter<W>) {
        (self.index_writer, self.position_writer, self.value_writer)
    }

    /// Creates a new writer.
    pub fn new(
        index_writer: index::Writer<V>,
        position_writer: PositionWriter<W>,
        value_writer: ValueWriter<W>,
    ) -> Self {
        Self {
            index_writer,
            position_writer,
            value_writer,
            index_record: None,
        }
    }

    /// Returns the inner position writer.
    pub fn position_writer(&self) -> &PositionWriter<W> {
        &self.position_writer
    }

    /// Returns a mutable reference to the inner position writer.
    pub fn position_writer_mut(&mut self) -> &mut PositionWriter<W> {
        &mut self.position_writer
    }

    /// Returns the inner value writer.
    pub fn value_writer(&self) -> &ValueWriter<W> {
        &self.value_writer
    }

    /// Returns a mutable reference to the inner value writer.
    pub fn value_writer_mut(&mut self) -> &mut ValueWriter<W> {
        &mut self.value_writer
    }

    /// Writes the magic numbers.
    pub fn write_magic(&mut self) -> io::Result<()> {
        self.position_writer
            .write_magic()
            .and_then(|_| self.value_writer.write_magic())
    }
}

impl Writer<io::BufWriter<fs::File>, io::BufWriter<fs::File>> {
    /// Creates a new writer from paths.
    ///
    /// Note that the constructed writer will not be a BGZF writer.
    /// To construct a BGZF writer from paths, see the
    /// [`BgzfWriter::from_bgzf_paths`] constructor.
    ///
    /// The magic number will be written to the paths.
    pub fn from_paths<P>(index_path: P, position_path: P, value_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index_writer = index::Writer::from_path(index_path)?;
        let position_writer = PositionWriter::from_path(position_path)?;
        let value_writer = ValueWriter::from_path(value_path)?;

        Ok(Self::new(index_writer, position_writer, value_writer))
    }
}

impl<V, W> BgzfWriter<V, W>
where
    V: io::Write,
    W: io::Write,
{
    /// Writes a single record.
    pub fn write_record<T>(&mut self, record: &Record<T>) -> io::Result<()>
    where
        T: AsRef<str>,
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
                let position_offset = u64::from(self.position_writer.get_ref().virtual_position());
                let value_offset = u64::from(self.value_writer.get_ref().virtual_position());

                let old = mem::replace(
                    index_record,
                    index::Record::new(contig_id.to_string(), 1, position_offset, value_offset),
                );

                self.index_writer.write_record(&old)?;
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
        self.value_writer.write_values(record.values())?;

        Ok(())
    }

    /// Finishes writing.
    pub fn finish(mut self) -> io::Result<(V, W, W)> {
        if let Some(record) = self.index_record {
            self.index_writer.write_record(&record)?;
        }

        Ok((
            self.index_writer.into_inner(),
            self.position_writer.into_inner().finish()?,
            self.value_writer.into_inner().finish()?,
        ))
    }
}

impl BgzfWriter<io::BufWriter<fs::File>, io::BufWriter<fs::File>> {
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
        let prefix = super::ext::prefix_from_member_path(&member_path).ok_or_else(|| {
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
    pub fn from_bgzf_paths<P>(index_path: P, position_path: P, value_path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let index_writer = index::Writer::from_path(index_path)?;
        let position_writer = PositionWriter::from_bgzf_path(position_path)?;
        let value_writer = ValueWriter::from_bgzf_path(value_path)?;

        Ok(Self::new(index_writer, position_writer, value_writer))
    }

    /// Creates a new BGZF writer from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and value files are named according to a shared
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
        let index_path = prefix.as_ref().with_extension(INDEX_EXT);
        let position_path = prefix.as_ref().with_extension(POSITIONS_FILE_EXT);
        let value_path = prefix.as_ref().with_extension(VALUES_FILE_EXT);

        Self::from_bgzf_paths(index_path, position_path, value_path)
    }
}
