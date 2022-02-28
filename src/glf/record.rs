//! GLF record.

use std::{error, fmt, io, num, ops, str};

mod genotype;
pub use genotype::Genotype;

const SEP: &str = ":";
const SIZE: usize = 10;

/// A GLF record.
///
/// A record consists of likelihoods for each possible diploid, diallelic
/// genotype. By convention, these are log-scaled and scaled to the most likely
/// genotype. Access to a the likelihood of a particular genotype is provided
/// via custom [`Genotype`] type.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Record([f64; SIZE]);

impl Record {
    /// Returns a slice containing the entire record.
    ///
    /// The order of likelihoods within the slice is encoded by [`Genotype`].
    #[inline]
    pub fn as_slice(&self) -> &[f64] {
        self.0.as_slice()
    }

    /// Returns a mutable slice containing the entire record.
    ///
    /// The order of likelihoods within the slice is encoded by [`Genotype`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        self.0.as_mut_slice()
    }

    /// Creates a new record.
    ///
    /// # Examples
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an array containining the entire record, consuming `self`.
    pub fn to_array(self) -> [f64; SIZE] {
        self.0
    }
}

impl AsRef<[f64; SIZE]> for Record {
    #[inline]
    fn as_ref(&self) -> &[f64; SIZE] {
        &self.0
    }
}

impl AsMut<[f64; SIZE]> for Record {
    #[inline]
    fn as_mut(&mut self) -> &mut [f64; SIZE] {
        &mut self.0
    }
}

impl From<[f64; SIZE]> for Record {
    #[inline]
    fn from(values: [f64; SIZE]) -> Self {
        Self(values)
    }
}

impl From<Record> for [f64; SIZE] {
    #[inline]
    fn from(record: Record) -> Self {
        record.to_array()
    }
}

impl ops::Index<Genotype> for Record {
    type Output = f64;

    #[inline]
    fn index(&self, index: Genotype) -> &Self::Output {
        self.0.index(index as usize)
    }
}

impl ops::IndexMut<Genotype> for Record {
    #[inline]
    fn index_mut(&mut self, index: Genotype) -> &mut Self::Output {
        self.0.index_mut(index as usize)
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0[0].fmt(f)?;

        for value in self.as_slice().iter().skip(1) {
            f.write_str(SEP)?;
            value.fmt(f)?;
        }

        Ok(())
    }
}

impl str::FromStr for Record {
    type Err = ParseRecordError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let values: [f64; SIZE] = s
            .splitn(SIZE, SEP)
            .map(f64::from_str)
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| ParseRecordError::MissingValues)?;

        Ok(Self::from(values))
    }
}

/// A record error associated with parsing a record.
#[derive(Debug)]
pub enum ParseRecordError {
    /// Record contained too few value.
    MissingValues,
    /// Record value failed to parse.
    ParseFloatError(num::ParseFloatError),
}

impl From<num::ParseFloatError> for ParseRecordError {
    fn from(error: num::ParseFloatError) -> Self {
        Self::ParseFloatError(error)
    }
}

impl fmt::Display for ParseRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseRecordError::MissingValues => f.write_str("missing values in record"),
            ParseRecordError::ParseFloatError(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for ParseRecordError {}

impl From<ParseRecordError> for io::Error {
    fn from(error: ParseRecordError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let record = Record::from([0., 1., 2., 3., 4., 5., 6., 7., 8., 9.]);
        assert_eq!(format!("{record:.0}"), "0:1:2:3:4:5:6:7:8:9");
        assert_eq!(
            format!("{record:.1}"),
            "0.0:1.0:2.0:3.0:4.0:5.0:6.0:7.0:8.0:9.0"
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            "0.:1.:2.:3.:4.:5.:6.:7.:8.:9.".parse::<Record>().unwrap(),
            Record::from([0., 1., 2., 3., 4., 5., 6., 7., 8., 9.]),
        );
    }
}
