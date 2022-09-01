use std::io;

use byteorder::{WriteBytesExt, LE};

/// An extension trait for writing.
pub trait WriterExt {
    /// Writes a single position.
    fn write_position(&mut self, position: u32) -> io::Result<()>;

    /// Write likelihoods.
    fn write_likelihoods(&mut self, likelihoods: &[f32]) -> io::Result<()>;
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
}
