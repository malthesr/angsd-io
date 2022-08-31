use std::{io, mem};

use byteorder::{ReadBytesExt, LE};

use crate::ReadStatus;

/// An extension trait for reading SAF positions.
pub trait ReaderExt {
    /// Read a single position.
    ///
    /// Returns `None` if reader is at end of file.
    fn read_position(&mut self) -> io::Result<Option<u32>>;

    /// Read values.
    fn read_values(&mut self, buf: &mut [f32]) -> io::Result<ReadStatus>;
}

impl<R> ReaderExt for R
where
    R: io::BufRead,
{
    fn read_position(&mut self) -> io::Result<Option<u32>> {
        // Modified from std::io::default_read_exact
        let mut arr = [0; mem::size_of::<u32>()];
        let mut buf = &mut arr[..];

        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        if buf.len() == 4 {
            Ok(None)
        } else if !buf.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to read position",
            ))
        } else {
            Ok(Some(u32::from_le_bytes(arr)))
        }
    }

    fn read_values(&mut self, buf: &mut [f32]) -> io::Result<ReadStatus> {
        if ReadStatus::check(self)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        self.read_f32_into::<LE>(buf).map(|_| ReadStatus::NotDone)
    }
}
