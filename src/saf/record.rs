//! SAF record.

use std::{error::Error, fmt, io, num, str::FromStr};

use byteorder::{ReadBytesExt, LE};

use crate::{
    saf::reader::{ReadableInto, ReaderExt},
    ReadStatus,
};

use super::{index::Index, Version};

const SEP: &str = "\t";

/// A SAF index contig ID.
///
/// The ID has no meaning other than that it may be used to index the SAF index records.
pub type Id = usize;

/// SAF likelihoods values.
pub type Likelihoods = Vec<f32>;

/// A SAF likelihood value band.
///
/// The value band describes the start of the band, as well as its length, and contains the
/// likelihoods within the band. All values outside the band are implicitly zero.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    start: usize,
    likelihoods: Likelihoods,
}

impl Band {
    /// Returns the band likelihoods, consuming `self`.
    pub fn into_likelihoods(self) -> Vec<f32> {
        self.likelihoods
    }

    /// Returns the length of the band.
    ///
    /// This correspond to the number of likelihoods in the band.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.likelihoods.len()
    }

    /// Creates a new band.
    pub fn new(start: usize, likelihoods: Likelihoods) -> Self {
        Self { start, likelihoods }
    }

    /// Returns the start of the band.
    ///
    /// This corresponds to the first sample frequency that is represented in the band.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns a mutable reference to the start of the band.
    pub fn start_mut(&mut self) -> &mut usize {
        &mut self.start
    }

    /// Returns a reference to the band values.
    pub fn likelihoods(&self) -> &[f32] {
        &self.likelihoods
    }

    /// Returns a mutable reference to the band values.
    pub fn likelihoods_mut(&mut self) -> &mut Vec<f32> {
        &mut self.likelihoods
    }
}

impl ReadableInto for Band {
    type Return = ReadStatus;

    fn read_into<R>(reader: &mut R, buf: &mut Self) -> io::Result<Self::Return>
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
            .read_values(buf.likelihoods_mut())
            .map(|_| ReadStatus::NotDone)
    }
}

impl ReadableInto for Likelihoods {
    type Return = ReadStatus;

    fn read_into<R>(reader: &mut R, buf: &mut Self) -> io::Result<Self::Return>
    where
        R: io::BufRead,
    {
        reader.read_values(buf)
    }
}

/// A SAF record.
///
/// The record is parameterised over the contig ID type and its contents. When reading, the contig
/// ID will be an [`Id`]. When writing, the contig ID will be string-like. The contents can either
/// a full set of [`Likelihoods`], or only a smaller [`Band`] of likelihoods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record<I, T> {
    contig_id: I,
    position: u32,
    contents: T,
}

impl<I, T> Record<I, T> {
    /// Returns the record contig ID.
    pub fn contig_id(&self) -> &I {
        &self.contig_id
    }

    /// Returns a mutable reference to the record contig ID.
    pub fn contig_id_mut(&mut self) -> &mut I {
        &mut self.contig_id
    }

    /// Returns the record contents, consuming `self`.
    pub fn into_contents(self) -> T {
        self.contents
    }

    /// Creates a new record.
    ///
    /// See also the [`Self::from_alleles`] constructor and the
    /// [`Writer::create_record_buf`](crate::saf::Reader::create_record_buf) convenience method.
    pub fn new(contig_id: I, position: u32, contents: T) -> Self {
        Self {
            contig_id,
            position,
            contents,
        }
    }

    /// Returns the record position.
    pub fn position(&self) -> u32 {
        self.position
    }

    /// Returns a mutable reference to the record position.
    pub fn position_mut(&mut self) -> &mut u32 {
        &mut self.position
    }

    /// Returns a reference to the record contents.
    pub fn contents(&self) -> &T {
        &self.contents
    }

    /// Returns a mutable reference to the record contents.
    pub fn contents_mut(&mut self) -> &mut T {
        &mut self.contents
    }
}

impl<I> Record<I, Likelihoods> {
    /// Returns the record alleles.
    ///
    /// This is equal to `2N` for `N` diploid individuals.
    pub fn alleles(&self) -> usize {
        self.contents.len() - 1
    }

    /// Creates a new record with a fixed number of zero-initialised values.
    pub fn from_alleles(contig_id: I, position: u32, alleles: usize) -> Self {
        let values = vec![0.0; alleles + 1];

        Self::new(contig_id, position, values)
    }
}

impl<T> Record<Id, T> {
    /// Creates a new record with a named contig ID, consuming `self`.
    ///
    /// # Panics
    ///
    /// If current contig ID is not found in `index`.
    pub fn to_named<V>(self, index: &Index<V>) -> Record<&str, T>
    where
        V: Version,
    {
        let name = index.records()[self.contig_id].name();

        Record::new(name, self.position, self.contents)
    }
}

impl<I> fmt::Display for Record<I, Likelihoods>
where
    I: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.contig_id)?;
        write!(f, "{SEP}{}", self.position)?;

        for value in self.contents().iter() {
            f.write_str(SEP)?;
            value.fmt(f)?;
        }

        Ok(())
    }
}

impl FromStr for Record<String, Likelihoods> {
    type Err = ParseRecordError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split_whitespace();

        let contig_id = iter
            .next()
            .ok_or(ParseRecordError::MissingContigId)?
            .to_string();

        let position = iter
            .next()
            .ok_or(ParseRecordError::MissingPosition)?
            .parse()
            .map_err(ParseRecordError::InvalidPosition)?;

        let contents = iter
            .map(|v| v.parse())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(ParseRecordError::InvalidValues)?;

        if !contents.is_empty() {
            Ok(Self::new(contig_id, position, contents))
        } else {
            Err(ParseRecordError::MissingValues)
        }
    }
}

/// An error associated with parsing a record.
#[derive(Debug)]
pub enum ParseRecordError {
    /// Record contig ID missing.
    MissingContigId,
    /// Record position missing.
    MissingPosition,
    /// Record position invalid.
    InvalidPosition(num::ParseIntError),
    /// Record values missing.
    MissingValues,
    /// Record values invalid.
    InvalidValues(num::ParseFloatError),
}

impl fmt::Display for ParseRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseRecordError::MissingContigId => f.write_str("missing record contig ID"),
            ParseRecordError::MissingPosition => f.write_str("missing record position"),
            ParseRecordError::InvalidPosition(e) => {
                write!(f, "failed to parse record position: '{e}'")
            }
            ParseRecordError::MissingValues => f.write_str("missing record values"),
            ParseRecordError::InvalidValues(e) => write!(f, "failed to parse record value: '{e}'"),
        }
    }
}

impl Error for ParseRecordError {}

impl From<ParseRecordError> for io::Error {
    fn from(error: ParseRecordError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }
}
