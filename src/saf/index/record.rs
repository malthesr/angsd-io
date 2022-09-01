use std::{fmt, io, marker::PhantomData};

use crate::saf::version::{Version, V3, V4};

/// A SAF index record.
///
/// Each index record corresponds to a contig contained in the associated SAF files.
///
/// The [`V3`] and [`V4`] records differ in whether the record contains the sum of band information
/// for the record in question.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record<V> {
    name: String,
    sites: usize,
    // We maintain the invariant is that `sum_band` is always `None` for V3 and `Some` for V4
    sum_band: Option<usize>,
    position_offset: u64,
    item_offset: u64,
    v: PhantomData<V>,
}

impl<V> Record<V>
where
    V: Version,
{
    /// Returns the reference sequence name, consuming `self`.
    pub fn into_name(self) -> String {
        self.name
    }

    /// Returns the item offset.
    ///
    /// This is the byte offset into the item file at which the reference sequence data begins.
    pub fn item_offset(&self) -> u64 {
        self.item_offset
    }

    /// Returns a mutable reference to the item offset.
    ///
    /// This is the byte offset into the item file at which the reference sequence data begins.
    pub fn item_offset_mut(&mut self) -> &mut u64 {
        &mut self.item_offset
    }

    /// Returns the reference sequence name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Returns a mutable reference to the reference sequence name.
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// Returns the position offset.
    ///
    /// This is the byte offset into the position file at which the reference sequence data begins.
    pub fn position_offset(&self) -> u64 {
        self.position_offset
    }

    /// Returns a mutable reference to the position offset.
    ///
    /// This is the byte offset into the position file at which the reference sequence data begins.
    pub fn position_offset_mut(&mut self) -> &mut u64 {
        &mut self.position_offset
    }

    /// Reads a record from a reader.
    ///
    /// The stream is assumed to be positioned immediately in front of a record.
    pub fn read<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        V::read_index_record(reader)
    }

    /// Returns the record sites.
    ///
    /// This is the number of sites on the reference sequence contained in the position and item
    /// files.
    pub fn sites(&self) -> usize {
        self.sites
    }

    /// Returns a mutable reference to the record sites.
    ///
    /// This is the number of sites on the reference sequence contained in the position and item
    /// files.
    pub fn sites_mut(&mut self) -> &mut usize {
        &mut self.sites
    }

    /// Writes a record to a writer.
    pub fn write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        V::write_index_record(writer, self)
    }
}

impl Record<V3> {
    /// Creates a new record.
    pub fn new(name: String, sites: usize, position_offset: u64, item_offset: u64) -> Self {
        Self {
            name,
            sites,
            sum_band: None,
            position_offset,
            item_offset,
            v: PhantomData,
        }
    }
}

impl Record<V4> {
    /// Creates a new record.
    pub fn new_with_sum_band(
        name: String,
        sites: usize,
        sum_band: usize,
        position_offset: u64,
        item_offset: u64,
    ) -> Self {
        Self {
            name,
            sites,
            sum_band: Some(sum_band),
            position_offset,
            item_offset,
            v: PhantomData,
        }
    }
    /// Returns the record sum of bands.
    pub fn sum_band(&self) -> usize {
        self.sum_band.unwrap()
    }

    /// Returns a mutable reference to the record sum of bands.
    pub fn sum_band_mut(&mut self) -> &mut usize {
        self.sum_band.as_mut().unwrap()
    }
}

impl fmt::Display for Record<V3> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "#contig=<ID={}, sites={}>", self.name, self.sites)
    }
}

impl fmt::Display for Record<V4> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "#contig=<ID={}, sites={}, sum_band={}>",
            self.name,
            self.sites,
            self.sum_band()
        )
    }
}
