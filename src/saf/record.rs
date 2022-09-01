//! SAF record.

use std::{error::Error, fmt, io, num, str::FromStr};

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
/// The band describes the start of the band, as well as its length, and contains the
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

    /// Returns a reference to the band likelihoods.
    pub fn likelihoods(&self) -> &[f32] {
        &self.likelihoods
    }

    /// Returns a mutable reference to the band likelihoods.
    pub fn likelihoods_mut(&mut self) -> &mut Vec<f32> {
        &mut self.likelihoods
    }
}

/// A SAF record.
///
/// The record is parameterised over the contig ID type and its contained item. When reading, the
/// ID will be an [`Id`]. When writing, the contig ID will be string-like. The contained item can
/// either be a full set of [`Likelihoods`], or only a smaller [`Band`] of likelihoods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record<I, T> {
    contig_id: I,
    position: u32,
    item: T,
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

    /// Returns the record item, consuming `self`.
    pub fn into_item(self) -> T {
        self.item
    }

    /// Returns a reference to the record item.
    pub fn item(&self) -> &T {
        &self.item
    }

    /// Returns a mutable reference to the record item.
    pub fn item_mut(&mut self) -> &mut T {
        &mut self.item
    }

    /// Creates a new record.
    pub fn new(contig_id: I, position: u32, item: T) -> Self {
        Self {
            contig_id,
            position,
            item,
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
}

impl<I> Record<I, Likelihoods> {
    /// Returns the record alleles.
    ///
    /// This is equal to `2N` for `N` diploid individuals.
    pub fn alleles(&self) -> usize {
        self.item.len() - 1
    }

    /// Creates a new record with a fixed number of zero-initialised likelihoods.
    pub fn from_alleles(contig_id: I, position: u32, alleles: usize) -> Self {
        let item = vec![0.0; alleles + 1];

        Self::new(contig_id, position, item)
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

        Record::new(name, self.position, self.item)
    }
}

impl<I> fmt::Display for Record<I, Likelihoods>
where
    I: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.contig_id)?;
        write!(f, "{SEP}{}", self.position)?;

        for v in self.item().iter() {
            f.write_str(SEP)?;
            v.fmt(f)?;
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

        let item = iter
            .map(|v| v.parse())
            .collect::<Result<Vec<f32>, _>>()
            .map_err(ParseRecordError::InvalidLikelihoods)?;

        if !item.is_empty() {
            Ok(Self::new(contig_id, position, item))
        } else {
            Err(ParseRecordError::MissingLikelihoods)
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
    MissingLikelihoods,
    /// Record values invalid.
    InvalidLikelihoods(num::ParseFloatError),
}

impl fmt::Display for ParseRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseRecordError::MissingContigId => f.write_str("missing record contig ID"),
            ParseRecordError::MissingPosition => f.write_str("missing record position"),
            ParseRecordError::InvalidPosition(e) => {
                write!(f, "failed to parse record position: '{e}'")
            }
            ParseRecordError::MissingLikelihoods => f.write_str("missing record likelihoods"),
            ParseRecordError::InvalidLikelihoods(e) => {
                write!(f, "failed to parse record likelihoods: '{e}'")
            }
        }
    }
}

impl Error for ParseRecordError {}

impl From<ParseRecordError> for io::Error {
    fn from(error: ParseRecordError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }
}
