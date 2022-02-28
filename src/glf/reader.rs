use std::{
    fs,
    io::{self, Read},
    mem,
    path::Path,
};

use byteorder::ReadBytesExt;

use crate::ReadStatus;

use super::{Endian, Record};

/// A BGZF GLF reader.
///
/// Note that this is a type alias for a [`Reader`], and most methods are
/// available via the [`Reader`] type.
pub type BgzfReader<R> = Reader<bgzf::Reader<R>>;

/// A GLF reader.
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::BufRead,
{
    /// Returns a mutable reference to the inner reader.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the inner reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Returns the inner reader, consuming `self.`
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Creates a new reader.
    ///
    /// Note that the constructed reader will not be a BGZF reader unless `R` is
    /// a BGZF reader. To construct a BGZF reader, see the
    /// [`BgzfReader::from_bgzf`] constructor.
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads a single record.
    pub fn read_record(&mut self, record: &mut Record) -> io::Result<ReadStatus> {
        if ReadStatus::check(&mut self.inner)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        read_record_unchecked(&mut self.inner, record).map(|()| ReadStatus::NotDone)
    }

    /// Reads multiple records.
    pub fn read_records(&mut self, records: &mut [Record]) -> io::Result<ReadStatus> {
        if ReadStatus::check(&mut self.inner)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        for record in records.iter_mut() {
            read_record_unchecked(&mut self.inner, record)?;
        }

        Ok(ReadStatus::NotDone)
    }

    /// Skips a single record.
    pub fn skip_record(&mut self) -> io::Result<ReadStatus> {
        self.skip_records(1)
    }

    /// Skips multiple records.
    pub fn skip_records(&mut self, records: usize) -> io::Result<ReadStatus> {
        if ReadStatus::check(&mut self.inner)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        skip_records_unchecked(&mut self.inner, records).map(|()| ReadStatus::NotDone)
    }

    /// Reads or skips multiple records.
    pub fn read_some_records(&mut self, records: &mut [Option<Record>]) -> io::Result<ReadStatus> {
        if ReadStatus::check(&mut self.inner)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        for maybe_record in records.iter_mut() {
            match maybe_record {
                Some(record) => read_record_unchecked(&mut self.inner, record)?,
                None => skip_records_unchecked(&mut self.inner, 1)?,
            }
        }

        Ok(ReadStatus::NotDone)
    }
}

impl Reader<io::BufReader<fs::File>> {
    /// Creates a new reader from a path.
    ///
    /// Note that the constructed reader will not be a BGZF reader.
    /// To construct a BGZF reader from path, see the
    /// [`BgzfReader::from_bgzf_path`] constructor.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::open(path).map(io::BufReader::new).map(Self::new)
    }
}

impl<R> BgzfReader<R>
where
    R: io::BufRead,
{
    /// Creates a new BGZF reader.
    ///
    /// This will wrap the inner reader `R` in a BGZF reader, so `R` should
    /// *not* be a BGZF reader.
    pub fn from_bgzf(inner: R) -> Self {
        Self::new(bgzf::Reader::new(inner))
    }
}

impl BgzfReader<io::BufReader<fs::File>> {
    /// Creates a new BGZF reader from a path.
    pub fn from_bgzf_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        fs::File::open(path)
            .map(io::BufReader::new)
            .map(Self::from_bgzf)
    }
}

impl<R> From<R> for Reader<R>
where
    R: io::BufRead,
{
    fn from(inner: R) -> Self {
        Self::new(inner)
    }
}

fn read_record_unchecked<R>(reader: &mut R, record: &mut Record) -> io::Result<()>
where
    R: io::BufRead,
{
    reader.read_f64_into::<Endian>(record.as_mut_slice())
}

fn skip_records_unchecked<R>(reader: &mut R, records: usize) -> io::Result<()>
where
    R: io::BufRead,
{
    let skip_bytes = (records * mem::size_of::<Record>()) as u64;

    io::copy(&mut reader.take(skip_bytes), &mut io::sink())?;

    Ok(())
}
