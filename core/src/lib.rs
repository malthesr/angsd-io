use std::io;

/// A read status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadStatus {
    /// The operation did not read any data from the underlying reader, and
    /// reading has finished.
    Done,
    /// The operation read data from the underlying reader, and reading may
    /// not be finished.
    NotDone,
}

impl ReadStatus {
    /// Returns the read status of a reader.
    ///
    /// # Returns
    ///
    /// Returns [`ReadStatus::Done`] if no more data remains to be read in the
    /// reader, otherwise returns [`ReadStatus::NotDone`] if any data remains.
    /// The reader may attempt to fill the underlying buffer to check for more
    /// data. An error in this processed is returned.
    pub fn check<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::BufRead,
    {
        // TODO: This can use io::BufRead::has_data_left if/when it stabilizes, see
        // tracking issue github.com/rust-lang/rust/issues/86423
        reader.fill_buf().map(|b| match b.is_empty() {
            true => Self::Done,
            false => Self::NotDone,
        })
    }

    /// Returns `true` if read status is [`ReadStatus::Done`].
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// Returns `true` if read status is [`ReadStatus::NotDone`].
    pub fn is_not_done(&self) -> bool {
        matches!(self, Self::NotDone)
    }
}
