use std::io;

const MAGIC_LEN: usize = 8;

/// A type that describes a SAF file version.
pub trait Version: Sized {
    /// The SAF version magic number.
    const MAGIC_NUMBER: [u8; MAGIC_LEN];

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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct V3;

impl Version for V3 {
    const MAGIC_NUMBER: [u8; MAGIC_LEN] = [b's', b'a', b'f', b'v', b'3', 0, 0, 0];
}
