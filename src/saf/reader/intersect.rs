use std::{cmp::Ordering, collections::HashMap, io};

use crate::{saf::IdRecord, ReadStatus};

use super::BgzfReader;

/// An intersection of BGZF SAF files readers.
pub struct Intersect<R> {
    left: BgzfReader<R>,
    right: BgzfReader<R>,
    left_to_right: Vec<Option<usize>>,
}

impl<R> Intersect<R>
where
    R: io::BufRead + io::Seek,
{
    /// Returns a new record pair suitable for use in reading.
    ///
    /// The [`Self::read_record_pair`] method requires a pair of record buffer with the correct
    /// number of alleles. This method creates such a record pair, using the number of alleles
    /// defined in the index.
    pub fn create_record_buf(&self) -> (IdRecord, IdRecord) {
        (
            self.left.create_record_buf(),
            self.right.create_record_buf(),
        )
    }

    /// Returns the left inner reader.
    pub fn get_left(&self) -> &BgzfReader<R> {
        &self.left
    }

    /// Returns a mutable reference to the left inner reader.
    pub fn get_left_mut(&self) -> &BgzfReader<R> {
        &self.left
    }

    /// Returns the right inner reader.
    pub fn get_right(&self) -> &BgzfReader<R> {
        &self.right
    }

    /// Returns a mutable reference to the right inner reader.
    pub fn get_right_mut(&self) -> &BgzfReader<R> {
        &self.right
    }

    /// Returns the inner readers, consuming `self`.
    pub fn into_parts(self) -> (BgzfReader<R>, BgzfReader<R>) {
        (self.left, self.right)
    }

    /// Creates a new intersection.
    pub fn new(left: BgzfReader<R>, right: BgzfReader<R>) -> Self {
        let right_name_to_id: HashMap<&str, usize> = right
            .index()
            .records()
            .iter()
            .enumerate()
            .map(|(i, rec)| (rec.name(), i))
            .collect();

        let left_to_right = left
            .index()
            .records()
            .iter()
            .map(|rec| right_name_to_id.get(rec.name()).copied())
            .collect();

        Self {
            left,
            right,
            left_to_right,
        }
    }

    /// Reads a single pair of intersecting records.
    ///
    /// If successful, a record from the left reader will be read into the left record, and a
    /// record from the right reader will be read into the right record.
    ///
    /// Note that `left` and `right` must have a number of values defined in accordance with the
    /// number of values in the corresponding SAF values files. See [`Self::create_record_buf`]
    ///  to create such records based on the provided indexes.
    pub fn read_record_pair(
        &mut self,
        left: &mut IdRecord,
        right: &mut IdRecord,
    ) -> io::Result<ReadStatus> {
        if self.left.read_record(left)?.is_done()
            || self.right.read_record(right)?.is_done()
            || self.read_until_shared_contig(left, right)?.is_done()
        {
            return Ok(ReadStatus::Done);
        }

        match self.read_until_shared_position_on_contig(left, right)? {
            Some(ReadStatus::Done) => Ok(ReadStatus::Done),
            Some(ReadStatus::NotDone) => Ok(ReadStatus::NotDone),
            None => self.read_record_pair(left, right),
        }
    }

    /// Reads records from the inner readers until they reach a shared contig.
    ///
    /// If `left` and `right` are already on the same contig, no further records are read.
    fn read_until_shared_contig(
        &mut self,
        left: &mut IdRecord,
        right: &mut IdRecord,
    ) -> io::Result<ReadStatus> {
        let mut left_id = *left.contig_id();

        loop {
            // Get the ID on the right that corresponds to the ID on the left;
            // this will be `None` if the left contig does not exist on the right.
            let corresponding_right_id = self.left_to_right[left_id];

            // Now we consider four cases:
            //
            // (1) The current left contig exists on the right, and...
            //     (1a) The current right contig is the same:
            //          we found an intersection, but not necessarily the last, so return `NotDone`
            //     (1b) The current right contig is not the same:
            //          seek the right reader to the left contig, read a right record, and continue
            // (2) The current left contig does not exist on the right, and...
            //     (2a) A later left contig does exist on the right:
            //          seek the left reader to this later contig, read a left record, and continue
            //     (2b) No later left contig exists on the right:
            //          no more intersecting records exist, return `Done`.
            if let Some(right_id) = corresponding_right_id {
                if right.contig_id() == &right_id {
                    // (1a)
                    return Ok(ReadStatus::NotDone);
                } else {
                    // (1b)
                    self.right.seek(right_id)?;
                    self.right.read_record(right)?;
                }
            } else if let Some(next_left_id) = self.left_to_right[left_id..]
                .iter()
                .position(|right_id| right_id.is_some())
            {
                // (2a)
                self.left.seek(next_left_id)?;
                self.left.read_record(left)?;
                left_id = *left.contig_id();
            } else {
                // (2b)
                return Ok(ReadStatus::Done);
            }
        }
    }

    /// Reads records from the inner reads until they reach a shared position, so long as they
    /// remain on the current contigs.
    ///
    /// If no more shared positions exist on contigs, returns `Some`.
    ///
    /// If `left` and `right` are already on the same position, no further records are read.
    fn read_until_shared_position_on_contig(
        &mut self,
        left: &mut IdRecord,
        right: &mut IdRecord,
    ) -> io::Result<Option<ReadStatus>> {
        let left_id = *left.contig_id();
        let right_id = *right.contig_id();

        let mut left_pos = left.position();
        let mut right_pos = right.position();

        loop {
            match left_pos.cmp(&right_pos) {
                Ordering::Less => {
                    if self.left.read_record(left)?.is_done() {
                        return Ok(Some(ReadStatus::Done));
                    }

                    left_pos = left.position();

                    if left.contig_id() != &left_id {
                        return Ok(None);
                    }
                }
                Ordering::Equal => return Ok(Some(ReadStatus::NotDone)),
                Ordering::Greater => {
                    if self.right.read_record(right)?.is_done() {
                        return Ok(Some(ReadStatus::Done));
                    }

                    right_pos = right.position();

                    if right.contig_id() != &right_id {
                        return Ok(None);
                    }
                }
            }
        }
    }
}
