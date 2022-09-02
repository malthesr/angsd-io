use std::io;

use byteorder::{WriteBytesExt, LE};

use crate::record::Band;

/// An extension trait for writing.
pub trait WriterExt {
    /// Writes a single position.
    fn write_position(&mut self, position: u32) -> io::Result<()>;

    /// Write likelihoods.
    fn write_likelihoods(&mut self, likelihoods: &[f32]) -> io::Result<()>;

    /// Write band.
    fn write_band(&mut self, band: &Band) -> io::Result<()>;
}

impl<W> WriterExt for W
where
    W: io::Write,
{
    fn write_position(&mut self, position: u32) -> io::Result<()> {
        self.write_u32::<LE>(position)
    }

    fn write_likelihoods(&mut self, likelihoods: &[f32]) -> io::Result<()> {
        for &v in likelihoods {
            self.write_f32::<LE>(v)?;
        }

        Ok(())
    }

    fn write_band(&mut self, band: &Band) -> io::Result<()> {
        let start = u32::try_from(band.start()).expect("cannot convert band start to u32");
        self.write_all(&start.to_le_bytes())?;

        let len =
            u32::try_from(band.likelihoods().len()).expect("cannot convert band length to u32");
        self.write_all(&len.to_le_bytes())?;

        self.write_likelihoods(band.likelihoods())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_band() -> io::Result<()> {
        let mut writer = Vec::new();
        let band = Band::new(8, vec![0., 1.]);
        writer.write_band(&band)?;

        assert_eq!(writer[0..4], 8u32.to_le_bytes());
        assert_eq!(writer[4..8], 2u32.to_le_bytes());
        assert_eq!(writer[8..12], 0.0f32.to_le_bytes());
        assert_eq!(writer[12..16], 1.0f32.to_le_bytes());

        Ok(())
    }
}
