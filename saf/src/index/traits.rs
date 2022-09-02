use std::{io, mem};

use crate::reader::ReaderExt;

/// An extension trait for reading indexes
pub trait IndexReaderExt: ReaderExt {
    /// Reads the number of allele categories for the index.
    ///
    /// This is a usize and follows immediately after the magic numbers in all supported formats.
    /// The stream is assumed to be positioned immediately before the alleles value.
    fn read_alleles(&mut self) -> io::Result<usize>;

    /// Reads the contig name of a next record.
    ///
    /// The stream is assumed to be positioned immediately before a the usize giving the number
    /// of characters in a record contig name.
    fn read_contig_name(&mut self) -> io::Result<String>;

    /// Reads the item offset of a record.
    fn read_item_offset(&mut self) -> io::Result<u64>;

    /// Reads the position offset of a record.
    fn read_position_offset(&mut self) -> io::Result<u64>;

    /// Reads the number of sites for a record.
    fn read_sites(&mut self) -> io::Result<usize>;

    /// Reads the sum of bins for a record.
    fn read_sum_band(&mut self) -> io::Result<usize>;
}

impl<R> IndexReaderExt for R
where
    R: io::BufRead,
{
    fn read_alleles(&mut self) -> io::Result<usize> {
        read_usize(self)
    }

    fn read_contig_name(&mut self) -> io::Result<String> {
        let mut usize_buf = [0; mem::size_of::<usize>()];
        self.read_exact(&mut usize_buf)?;
        let name_len = usize::from_le_bytes(usize_buf);

        let mut name_buf = vec![0; name_len];
        self.read_exact(&mut name_buf)?;
        String::from_utf8(name_buf).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "index record name not valid UTF8",
            )
        })
    }

    fn read_item_offset(&mut self) -> io::Result<u64> {
        read_u64(self)
    }

    fn read_position_offset(&mut self) -> io::Result<u64> {
        read_u64(self)
    }

    fn read_sites(&mut self) -> io::Result<usize> {
        read_usize(self)
    }

    fn read_sum_band(&mut self) -> io::Result<usize> {
        read_usize(self)
    }
}

/// An extension trait for writing indexes
pub trait IndexWriterExt {
    /// Writes the number of allele categories for the index.
    ///
    /// This is a usize and follows immediately after the magic numbers in all supported formats.
    fn write_alleles(&mut self, alleles: usize) -> io::Result<()>;

    /// Writes the contig name of a next record.
    fn write_contig_name(&mut self, contig_name: &str) -> io::Result<()>;

    /// Write the item offset of a record.
    fn write_item_offset(&mut self, item_offset: u64) -> io::Result<()>;

    /// Writes the position offset of a record.
    fn write_position_offset(&mut self, position_offset: u64) -> io::Result<()>;

    /// Writes the number of sites for a record.
    fn write_sites(&mut self, sites: usize) -> io::Result<()>;

    /// Writes the sum of bins for a record.
    fn write_sum_band(&mut self, sum_band: usize) -> io::Result<()>;
}

impl<W> IndexWriterExt for W
where
    W: io::Write,
{
    fn write_alleles(&mut self, alleles: usize) -> io::Result<()> {
        write_usize(self, alleles)
    }

    fn write_contig_name(&mut self, contig_name: &str) -> io::Result<()> {
        let raw_name = contig_name.as_bytes();
        write_usize(self, raw_name.len())?;
        self.write_all(raw_name)
    }

    fn write_item_offset(&mut self, item_offset: u64) -> io::Result<()> {
        write_u64(self, item_offset)
    }

    fn write_position_offset(&mut self, position_offset: u64) -> io::Result<()> {
        write_u64(self, position_offset)
    }

    fn write_sites(&mut self, sites: usize) -> io::Result<()> {
        write_usize(self, sites)
    }

    fn write_sum_band(&mut self, sum_band: usize) -> io::Result<()> {
        write_usize(self, sum_band)
    }
}

fn read_usize<R>(reader: &mut R) -> io::Result<usize>
where
    R: io::BufRead,
{
    let mut buf = [0; mem::size_of::<usize>()];
    reader.read_exact(&mut buf)?;

    Ok(usize::from_le_bytes(buf))
}

fn read_u64<R>(reader: &mut R) -> io::Result<u64>
where
    R: io::BufRead,
{
    let mut buf = [0; mem::size_of::<u64>()];
    reader.read_exact(&mut buf)?;

    Ok(u64::from_le_bytes(buf))
}

fn write_usize<W>(writer: &mut W, v: usize) -> io::Result<()>
where
    W: io::Write,
{
    writer.write_all(&v.to_le_bytes())
}

fn write_u64<W>(writer: &mut W, v: u64) -> io::Result<()>
where
    W: io::Write,
{
    writer.write_all(&v.to_le_bytes())
}
