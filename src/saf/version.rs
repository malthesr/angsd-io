use std::{io, mem};

use byteorder::{ReadBytesExt, LE};

use crate::ReadStatus;

use super::{
    index::{self, Index, IndexReaderExt, IndexWriterExt},
    reader::{Reader, ReaderExt},
    record::{Band, Id, Likelihoods, Record},
    writer::{Writer, WriterExt},
};

const MAGIC_LEN: usize = 8;

/// A type that describes a SAF file version.
///
/// Users should not generally need to use methods defined by this trait directly. Rather, these
/// methods are used by struct generic over methods instead.
///
/// Note that the SAF versioning is ambiguous: the versions described in the magic numbers are
/// out of sync with those used internally in ANGSD. The usage here follows the magic numbers.
/// Hence, what is here referred to as [`V3`] corresponds to files with magic numbers `safv3`,
/// which is also sometimes referred to as "Version 1" in ANGSD. Likewise, what is here referred to
/// as [`V4`] corresponds to files with magic numbers `safv4`, also sometimes known as "Version 2"
/// in ANGSD.
pub trait Version: Sized {
    /// The numeric description of the SAF version.
    const VERSION: u8;

    /// The SAF version magic number.
    const MAGIC_NUMBER: [u8; MAGIC_LEN];

    /// The items contained in the SAF item file for this version.
    type Item;

    /// Creates a SAF record buffer suitable for reading from a reader for this version.
    fn create_record_buf(index: &Index<Self>) -> Record<Id, Self::Item>;

    /// Reads the SAF index record for this version from a reader.
    fn read_index_record<R>(reader: &mut R) -> io::Result<index::Record<Self>>
    where
        R: io::BufRead;

    /// Reads a single item from a reader into a provided buffer.
    ///
    /// The stream is assumed to be positioned immediately before the start of the item.
    fn read_item<R>(reader: &mut R, buf: &mut Self::Item) -> io::Result<ReadStatus>
    where
        R: io::BufRead;

    /// Reads a single record from a SAF reader into a provided buffer.
    ///
    /// The stream is assumed to be positioned immediately before the start of the record.
    ///
    /// Note that the record buffer needs to be correctly set up. Use [`Self::create_record_buf`]
    /// for a correctly initialised record buffer to use for reading.
    fn read_record<R>(
        reader: &mut Reader<R, Self>,
        buf: &mut Record<Id, Self::Item>,
    ) -> io::Result<ReadStatus>
    where
        R: io::BufRead,
    {
        Reader::read_record(reader, buf)
    }

    /// Writes the SAF index record for to a reader.
    fn write_index_record<W>(writer: &mut W, record: &index::Record<Self>) -> io::Result<()>
    where
        W: io::Write;

    /// Writes a single item to a writer.
    fn write_item<W>(writer: &mut W, item: &Self::Item) -> io::Result<()>
    where
        W: io::Write;

    /// Writes a single record to a writer.
    fn write_record<W, I>(
        writer: &mut Writer<W, Self>,
        record: &Record<I, Self::Item>,
    ) -> io::Result<()>
    where
        W: io::Write,
        I: AsRef<str>;

    /// Reads the SAF version magic number from a reader.
    fn read_magic<R>(reader: &mut R) -> io::Result<()>
    where
        R: io::BufRead,
    {
        let mut magic = [0; MAGIC_LEN];
        reader.read_exact(&mut magic)?;

        if magic == Self::MAGIC_NUMBER {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "invalid or unsupported SAF magic number \
                    (found '{magic:02x?}', expected '{:02x?}')",
                    Self::MAGIC_NUMBER
                ),
            ))
        }
    }

    /// Writes the SAF version magic number to a writer.
    fn write_magic<W>(writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_all(&Self::MAGIC_NUMBER)
    }
}

/// A marker type for the SAF version 3.
///
/// In this version, the SAF item file contains the full set of likelihoods for each sample
/// frequency.
///
/// See also [`Version`] for a note on naming of versions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct V3;

impl Version for V3 {
    const VERSION: u8 = 3;

    const MAGIC_NUMBER: [u8; MAGIC_LEN] = [b's', b'a', b'f', b'v', b'3', 0, 0, 0];

    type Item = Likelihoods;

    fn create_record_buf(index: &Index<Self>) -> Record<Id, Self::Item> {
        // Record likelihoods must be set up to be correct size from beginning
        Record::from_alleles(0, 1, index.alleles())
    }

    fn read_index_record<R>(reader: &mut R) -> io::Result<index::Record<Self>>
    where
        R: io::BufRead,
    {
        let name = reader.read_contig_name()?;
        let sites = reader.read_sites()?;
        let position_offset = reader.read_position_offset()?;
        let item_offset = reader.read_item_offset()?;

        Ok(index::Record::new(
            name,
            sites,
            position_offset,
            item_offset,
        ))
    }

    fn read_item<R>(reader: &mut R, buf: &mut Self::Item) -> io::Result<ReadStatus>
    where
        R: io::BufRead,
    {
        reader.read_likelihoods(buf)
    }

    fn write_index_record<W>(writer: &mut W, record: &index::Record<Self>) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_contig_name(record.name())?;
        writer.write_sites(record.sites())?;
        writer.write_position_offset(record.position_offset())?;
        writer.write_item_offset(record.item_offset())
    }

    fn write_item<W>(writer: &mut W, item: &Self::Item) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_likelihoods(item)
    }

    fn write_record<W, I>(
        writer: &mut Writer<W, Self>,
        record: &Record<I, Self::Item>,
    ) -> io::Result<()>
    where
        W: io::Write,
        I: AsRef<str>,
    {
        let contig_id = record.contig_id().as_ref();

        // Handle index according to three cases:
        //
        // (1) New record is not the first, and...
        //     (1a) it is on a new contig: write the current index record and setup next
        //     (1b) is on the old contig: increment the count of sites on contig
        // (2) New record is the first: write alleles to index, and set up first index record
        if let Some(index_record) = writer.index_record.as_mut() {
            // Case (1)
            if index_record.name() != contig_id {
                // Case (1a)
                let position_offset = u64::from(writer.position_writer.virtual_position());
                let item_offset = u64::from(writer.item_writer.virtual_position());

                let old = mem::replace(
                    index_record,
                    index::Record::new(contig_id.to_string(), 1, position_offset, item_offset),
                );

                old.write(&mut writer.index_writer)?;
            } else {
                // Case (1b)
                *index_record.sites_mut() += 1;
            }
        } else {
            // Case (2)
            writer.index_writer.write_alleles(record.alleles())?;

            let offset = Self::MAGIC_NUMBER.len() as u64;
            writer.index_record =
                Some(index::Record::new(contig_id.to_string(), 1, offset, offset));
        }

        // Write record
        writer.position_writer.write_position(record.position())?;
        Self::write_item(&mut writer.item_writer, record.item())?;

        Ok(())
    }
}

/// A marker type for the SAF version 4.
///
/// In this version, the SAF item file contains only a smaller "band" of likelihoods centered around
/// the most likely sample frequency, along with information about the location of the band.
///
/// See also [`Version`] for a note on naming of versions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct V4;

impl Version for V4 {
    const VERSION: u8 = 4;

    const MAGIC_NUMBER: [u8; MAGIC_LEN] = [b's', b'a', b'f', b'v', b'3', 0, 0, 0];

    type Item = Band;

    fn create_record_buf(_index: &Index<Self>) -> Record<Id, Self::Item> {
        // Band is resized during reading, so we can simplify initialise empty band
        Record::new(0, 1, Band::new(0, Vec::new()))
    }

    fn read_index_record<R>(reader: &mut R) -> io::Result<index::Record<Self>>
    where
        R: io::BufRead,
    {
        let name = reader.read_contig_name()?;
        let sites = reader.read_sites()?;
        let sum_band = reader.read_sum_band()?;
        let position_offset = reader.read_position_offset()?;
        let item_offset = reader.read_item_offset()?;

        Ok(index::Record::new_with_sum_band(
            name,
            sites,
            sum_band,
            position_offset,
            item_offset,
        ))
    }

    fn read_item<R>(reader: &mut R, buf: &mut Self::Item) -> io::Result<ReadStatus>
    where
        R: io::BufRead,
    {
        if ReadStatus::check(reader)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        *buf.start_mut() = reader
            .read_u32::<LE>()?
            .try_into()
            .expect("cannot convert band start to usize");

        let len: usize = reader
            .read_u32::<LE>()?
            .try_into()
            .expect("cannot convert band length to usize");

        buf.likelihoods_mut().resize(len, 0.0);

        reader
            .read_likelihoods(buf.likelihoods_mut())
            .map(|_| ReadStatus::NotDone)
    }

    fn write_index_record<W>(writer: &mut W, record: &index::Record<Self>) -> io::Result<()>
    where
        W: io::Write,
    {
        writer.write_contig_name(record.name())?;
        writer.write_sites(record.sites())?;
        writer.write_sum_band(record.sum_band())?;
        writer.write_position_offset(record.position_offset())?;
        writer.write_item_offset(record.item_offset())
    }

    fn write_item<W>(writer: &mut W, item: &Self::Item) -> io::Result<()>
    where
        W: io::Write,
    {
        todo!()
    }

    fn write_record<W, I>(
        writer: &mut Writer<W, Self>,
        record: &Record<I, Self::Item>,
    ) -> io::Result<()>
    where
        W: io::Write,
        I: AsRef<str>,
    {
        todo!()
    }
}
