use std::{cmp::Ordering, io};

use indexmap::IndexMap;

use crate::{
    record::{Id, Record},
    version::Version,
    ReadStatus,
};

use super::{Index, Reader};

/// An intersection of SAF file readers.
///
/// The intersection takes an arbitrary number of readers and returns data where all readers
/// contain data for the same contig and position. It is assumed that contigs are sorted in the
/// same order in each file, and that positions are sorted numerically within each contig.
pub struct Intersect<R, V> {
    readers: Vec<Reader<R, V>>,
    shared_contigs: SharedContigs,
    ids: Vec<usize>, // Current reader contig IDs
}

impl<R, V> Intersect<R, V>
where
    R: io::BufRead + io::Seek,
    V: Version,
{
    /// Returns a new collection of records suitable for use in reading.
    pub fn create_record_bufs(&self) -> Vec<Record<Id, V::Item>> {
        self.readers
            .iter()
            .map(|reader| reader.create_record_buf())
            .collect()
    }

    /// Creates a new intersecting reader with an additional reader, consuming `self`.
    ///
    /// Since `self` is consumed, rather than mutated, this can be chained to build intersections
    /// of multiple readers. See also the [`Reader::intersect`] method for a way to start create
    /// the initial intersecting reader.
    pub fn intersect(mut self, reader: Reader<R, V>) -> Self {
        self.shared_contigs.add_index(reader.index());
        self.readers.push(reader);
        self.ids.push(0);
        self
    }

    /// Returns the inner readers.
    pub fn get_readers(&self) -> &[Reader<R, V>] {
        &self.readers
    }

    /// Returns a mutable reference to the inner readers.
    pub fn get_readers_mut(&mut self) -> &mut [Reader<R, V>] {
        &mut self.readers
    }

    /// Returns the inner readers, consuming `self`.
    pub fn into_readers(self) -> Vec<Reader<R, V>> {
        self.readers
    }

    /// Creates a new intersecting reader from a collection of readers.
    ///
    /// # Panics
    ///
    /// Panics if `readers` is empty.
    pub fn new(readers: Vec<Reader<R, V>>) -> Self {
        match readers.as_slice() {
            [] => panic!("cannot construct empty intersection"),
            [fst, tl @ ..] => {
                let mut contigs = SharedContigs::from(fst.index());
                for reader in tl.iter() {
                    contigs.add_index(reader.index());
                }

                let ids = vec![0; readers.len()];

                Self {
                    readers,
                    shared_contigs: contigs,
                    ids,
                }
            }
        }
    }

    /// Reads a set of intersecting records, one from each contained reader.
    ///
    /// If successful, a record from each inner reader will be read into the corresponding buffer
    /// such that all resulting records will be on the same contig and the same position.
    ///
    /// Note that the record buffer needs to be correctly set up. Use [`Self::create_record_bufs`]
    /// for a correctly initialised record buffers to use for reading.
    pub fn read_records(&mut self, bufs: &mut [Record<Id, V::Item>]) -> io::Result<ReadStatus> {
        for ((reader, record), id) in self
            .readers
            .iter_mut()
            .zip(bufs.iter_mut())
            .zip(self.ids.iter_mut())
        {
            if reader.read_record(record)?.is_done() {
                return Ok(ReadStatus::Done);
            }

            *id = *record.contig_id();
        }

        if self.read_until_shared_contig(bufs)?.is_done() {
            return Ok(ReadStatus::Done);
        }

        match self.read_until_shared_position_on_contig(bufs)? {
            Some(ReadStatus::Done) => Ok(ReadStatus::Done),
            Some(ReadStatus::NotDone) => Ok(ReadStatus::NotDone),
            None => self.read_records(bufs),
        }
    }

    pub(super) fn from_reader(reader: Reader<R, V>) -> Self {
        Self {
            shared_contigs: SharedContigs::from(reader.index()),
            readers: vec![reader],
            ids: vec![0],
        }
    }

    /// Read all readers until they are on a shared contig equal to or after the contigs defined
    /// by the provided record buffers.
    ///
    /// If no more shared contigs exist, returns `Done`.
    fn read_until_shared_contig(
        &mut self,
        bufs: &mut [Record<Id, V::Item>],
    ) -> io::Result<ReadStatus> {
        // For each record, get the first shared contig (by index into shared_contigs)
        // equal to or after the contig ID of that record. Then get the greatest/most distant of
        // those shared contigs. We can safely seek to this contig. If in the process, we find
        // that a reader has no such shared contigs, we are done.
        let mut next_idx = 0;
        for (reader, buf) in self.readers.iter_mut().zip(bufs.iter_mut()) {
            match self
                .shared_contigs
                .next_shared(reader.index(), *buf.contig_id())
            {
                Some(idx) => {
                    if idx > next_idx {
                        next_idx = idx;
                    }
                }
                None => return Ok(ReadStatus::Done),
            }
        }

        // Seek all readers to candidate shared contig, if they are not on it already.
        let next_ids = &self.shared_contigs.0[next_idx];
        for (((reader, buf), next_id), id) in self
            .readers
            .iter_mut()
            .zip(bufs.iter_mut())
            .zip(next_ids.iter())
            .zip(self.ids.iter_mut())
        {
            if buf.contig_id() != next_id {
                reader.seek(*next_id)?;
                reader.read_record(buf)?;
                *id = *next_id;
            }
        }

        Ok(ReadStatus::NotDone)
    }

    /// Reads all readers until they are on a shared position on the current contig.
    ///
    /// The starting read positions will be defined by the provided buffers; the current contig is
    /// defined by `self.ids`.
    ///
    /// If no more shared positions exist, returns `Some(Done)`. If more shared positions may exist,
    /// but not on the current contig, returns `None`. If `Some(NotDone)` is returned, an
    /// intersecting positions has been found.
    fn read_until_shared_position_on_contig(
        &mut self,
        bufs: &mut [Record<Id, V::Item>],
    ) -> io::Result<Option<ReadStatus>> {
        let mut max_pos = bufs
            .iter()
            .map(Record::position)
            .max()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty buffer slice"))?;

        'outer: loop {
            // We keep checking if all the records have reached the max position:
            // if so, we have a shared record. If we find one greater than max, max is updated.
            // As we go, we have to check that we have not reached a new contig ID.
            'inner: for ((reader, record), id) in self
                .readers
                .iter_mut()
                .zip(bufs.iter_mut())
                .zip(self.ids.iter_mut())
            {
                let mut pos = record.position();

                // A shared position must be at least as great as the max among all records
                match pos.cmp(&max_pos) {
                    Ordering::Less => {
                        // If a position is less than the current max, we can forward the
                        // corresponding reader all the way to its first position equal to or
                        // greater than the current max
                        while pos < max_pos {
                            if reader.read_record(record)?.is_done() {
                                return Ok(Some(ReadStatus::Done));
                            }
                            if record.contig_id() != id {
                                *id = *record.contig_id();
                                return Ok(None);
                            }
                            pos = record.position();
                        }

                        if pos == max_pos {
                            continue 'inner;
                        } else {
                            // Forward overshot current max_pos, which means we have to start over
                            // checking all records
                            continue 'outer;
                        }
                    }
                    Ordering::Equal => (),
                    Ordering::Greater => {
                        max_pos = pos;
                        continue 'outer;
                    }
                }
            }

            // To have reached this point, all record positions are at the current max, which means
            // an intersection
            return Ok(Some(ReadStatus::NotDone));
        }
    }
}

/// Shared contigs for readers based on their indexes.
///
/// The representation used is an ordered map from contig names to a vector of contig IDs,
/// corresponding to the contig IDs used in the represented reader. For instance, if "chr1"
/// is the first shared contig among two represented reads, and the first entry in the map is
/// ("chr1", vec![1, 2]), then "chr1" has ID 1 in the first reader, and ID 2 in the second reader.
/// As elsewhere here, the ID is based on the position in the index.
///
/// Note that as for `Intersect` generally, we assume that contigs occur in the same order in each
/// index. That is, the same contigs may not be represented in each index, and the same contig may
/// have a different IDs, but where two or more contigs occur in multiple indices, their ordering
/// must be constant.
#[derive(Clone, Debug)]
struct SharedContigs(IndexMap<String, Vec<usize>>);

impl SharedContigs {
    /// Adds a new index to the collection.
    ///
    /// The new index will be placed last.
    pub fn add_index<V>(&mut self, index: &Index<V>)
    where
        V: Version,
    {
        // Create a temporary mapping from names to IDs in the new index
        let map: IndexMap<&str, usize> = index
            .records()
            .iter()
            .enumerate()
            .map(|(i, record)| (record.name(), i))
            .collect();

        // For each name in the current selection, check if the name exists in the new index:
        // if so, (1) add its ID in the new index to the collection of IDs for this shared contig;
        // if not, (2) the contig is no longer shared, and should be removed
        self.0.retain(|name, ids| {
            if let Some(new_id) = map.get(name.as_str()) {
                // (1)
                ids.push(*new_id);
                true
            } else {
                // (2)
                false
            }
        })
    }

    /// Returns the index in `self` of the first contig in `index` with ID equal to or greater
    /// than `id`.
    ///
    /// If no more shared contigs exist in `index` after `id`, return `None`.
    pub fn next_shared<V>(&self, index: &Index<V>, id: usize) -> Option<usize>
    where
        V: Version,
    {
        let name = index.records()[id].name();

        self.0.get_index_of(name).or_else(|| {
            index.records()[(id + 1)..]
                .iter()
                .find_map(|record| self.0.get_index_of(record.name()))
        })
    }
}

impl<V> From<&Index<V>> for SharedContigs
where
    V: Version,
{
    fn from(index: &Index<V>) -> Self {
        index
            .records()
            .iter()
            .enumerate()
            .map(|(i, record)| (record.name().to_owned(), vec![i]))
            .collect()
    }
}

impl FromIterator<(String, Vec<usize>)> for SharedContigs {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (String, Vec<usize>)>,
    {
        Self(iter.into_iter().map(|(s, a)| (s, a)).collect())
    }
}
