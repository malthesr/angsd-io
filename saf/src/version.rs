//! SAF file versions.
//!
//! This module provides types and traits for abstracting over different SAF files version.
//! Users should not generally need to interact with code in this module, except perhaps to use the
//! marker structs in generic bounds.
//!
//! Note that the SAF versioning is ambiguous: the versions described in the magic numbers are
//! out of sync with those used internally in ANGSD. The usage here follows the magic numbers.
//! Hence, what is here referred to as [`V3`] corresponds to files with magic numbers `safv3`,
//! which is also sometimes referred to as "Version 1" in ANGSD. Likewise, what is here referred to
//! as [`V4`] corresponds to files with magic numbers `safv4`, also sometimes known as "Version 2"
//! in ANGSD.

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

        if let Some(index_record) = writer.index_record.as_mut() {
            if index_record.name() == contig_id {
                // We're on the same contig, so we can simply update index record
                *index_record.sites_mut() += 1;
            } else {
                // We're on a new contig, which means we have to write the current record index
                // and set up a new one
                let position_offset = u64::from(writer.position_writer.virtual_position());
                let item_offset = u64::from(writer.item_writer.virtual_position());

                let new =
                    index::Record::new(contig_id.to_string(), 1, position_offset, item_offset);

                let old = mem::replace(index_record, new);
                old.write(&mut writer.index_writer)?;
            }
        } else {
            let offset = Self::MAGIC_NUMBER.len() as u64;
            let index_record = index::Record::new(contig_id.to_string(), 0, offset, offset);
            writer.index_record = Some(index_record);

            return Self::write_record(writer, record);
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

    const MAGIC_NUMBER: [u8; MAGIC_LEN] = [b's', b'a', b'f', b'v', b'4', 0, 0, 0];

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
        writer.write_band(item)
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

        if let Some(index_record) = writer.index_record.as_mut() {
            if index_record.name() == contig_id {
                // We're on the same contig, so we can simply update index record
                *index_record.sum_band_mut() += record.item().likelihoods().len();
                *index_record.sites_mut() += 1;
            } else {
                // We're on a new contig, which means we have to write the current record index
                // and set up a new one
                let position_offset = u64::from(writer.position_writer.virtual_position());
                let item_offset = u64::from(writer.item_writer.virtual_position());

                let new = index::Record::new_with_sum_band(
                    contig_id.to_string(),
                    1,
                    0,
                    position_offset,
                    item_offset,
                );

                let old = mem::replace(index_record, new);
                old.write(&mut writer.index_writer)?;
            }
        } else {
            let offset = Self::MAGIC_NUMBER.len() as u64;
            let index_record =
                index::Record::new_with_sum_band(contig_id.to_string(), 0, 0, offset, offset);
            writer.index_record = Some(index_record);

            return Self::write_record(writer, record);
        }

        // Write record
        writer.position_writer.write_position(record.position())?;
        Self::write_item(&mut writer.item_writer, record.item())?;

        Ok(())
    }
}
