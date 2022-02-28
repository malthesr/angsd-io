//! SAF record.

use std::{error::Error, fmt, io, num, str::FromStr};

use super::Index;

const SEP: &str = "\t";

/// A SAF record with a contig ID referencing an index in
/// [`Index::records`];
///
/// See [`Record`] for details, and [`IdRecord::to_named`] for conversion.
pub type IdRecord = Record<usize>;

/// A SAF record.
///
/// The record is parameterised over the contig ID type. When reading, the contig ID will be a
/// usize, which may be used to index the index records, i.e. may be used to index the result of
/// [`Index::records`]. The [`IdRecord`] type alias is provided for this case.
/// When writing, the contig ID will be a string-like.
#[derive(Clone, Debug, PartialEq)]
pub struct Record<T> {
    contig_id: T,
    position: u32,
    values: Box<[f32]>,
}

impl<T> Record<T> {
    /// Returns the record alleles.
    ///
    /// This is equal to `2N` for `N` diploid individuals.
    pub fn alleles(&self) -> usize {
        self.values.len() - 1
    }

    /// Returns the record contig ID.
    pub fn contig_id(&self) -> &T {
        &self.contig_id
    }

    /// Returns a mutable reference to the record contig ID.
    pub fn contig_id_mut(&mut self) -> &mut T {
        &mut self.contig_id
    }

    /// Creates a new record with a fixed number of zero-initialised values.
    pub fn from_alleles(contig_id: T, position: u32, alleles: usize) -> Self {
        let values = vec![0.0; alleles + 1].into_boxed_slice();

        Self::new(contig_id, position, values)
    }

    /// Returns the record values, consuming `self`.
    pub fn into_values(self) -> Box<[f32]> {
        self.values
    }

    /// Creates a new record.
    ///
    /// See also the [`Self::from_alleles`] constructor and the
    /// [`Writer::create_record_buf`](crate::saf::Reader::create_record_buf) convenience method.
    pub fn new(contig_id: T, position: u32, values: Box<[f32]>) -> Self {
        Self {
            contig_id,
            position,
            values,
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

    /// Returns a reference to the record values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Returns a mutable reference to the record values.
    pub fn values_mut(&mut self) -> &mut [f32] {
        &mut self.values
    }
}

impl IdRecord {
    /// Creates a new record with a named contig ID, consuming `self`.
    ///
    /// # Panics
    ///
    /// If current contig ID is not found in `index`.
    pub fn to_named(self, index: &Index) -> Record<&str> {
        let name = index.records()[self.contig_id].name();

        Record::new(name, self.position, self.values)
    }
}

impl<T> fmt::Display for Record<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.contig_id)?;
        write!(f, "{SEP}{}", self.position)?;

        for value in self.values().iter() {
            f.write_str(SEP)?;
            value.fmt(f)?;
        }

        Ok(())
    }
}

impl FromStr for Record<String> {
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

        let values = iter
            .map(|v| v.parse())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(ParseRecordError::InvalidValues)?
            .into_boxed_slice();

        if values.len() > 0 {
            Ok(Self::new(contig_id, position, values))
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
