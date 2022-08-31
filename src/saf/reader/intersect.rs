use std::{cmp::Ordering, io};

use indexmap::IndexMap;

use crate::{
    saf::{Version, V3},
    ReadStatus,
};

use super::{BgzfReader, IdRecord, Index};

/// An intersection of BGZF SAF file readers.
///
/// Created by the [`BgzfReader::intersect`] method.
pub struct Intersect<R, V: Version = V3> {
    readers: Vec<BgzfReader<R, V>>,
    contigs: Contigs,
    ids: Vec<usize>,
}

impl<R, V> Intersect<R, V>
where
    R: io::BufRead + io::Seek,
    V: Version,
{
    /// Returns a new collection of records suitable for use in reading.
    ///
    /// The [`Self::read_records`] method requires a collection of record buffers of the correct
    /// length and with the correct number of alleles. This method creates such a record collection,
    /// using the number of alleles defined in the indexes.
    pub fn create_record_bufs(&self) -> Vec<IdRecord> {
        self.readers
            .iter()
            .map(|reader| reader.create_record_buf())
            .collect()
    }

    /// Creates a new intersecting reader with an additional reader, consuming `self`.
    ///
    /// Since `self` is consumed, rather than mutated, this can be chained to build intersections
    /// of multiple readers. See also the [`BgzfReader::intersect`] method for a way to start create
    /// the initial intersecting reader.
    pub fn intersect(mut self, reader: BgzfReader<R, V>) -> Self {
        self.contigs.add_index(reader.index());
        self.readers.push(reader);
        self.ids.push(0);
        self
    }

    /// Returns the inner readers.
    pub fn get_readers(&self) -> &[BgzfReader<R, V>] {
        &self.readers
    }

    /// Returns a mutable reference to the inner readers.
    pub fn get_readers_mut(&mut self) -> &mut [BgzfReader<R, V>] {
        &mut self.readers
    }

    /// Returns the inner readers, consuming `self`.
    pub fn into_readers(self) -> Vec<BgzfReader<R, V>> {
        self.readers
    }

    /// Creates a new intersecting reader from a collection of readers.
    ///
    /// # Panics
    ///
    /// Panics if `readers` is empty.
    pub fn new(readers: Vec<BgzfReader<R, V>>) -> Self {
        match readers.as_slice() {
            [] => panic!("cannot construct empty intersection"),
            [fst, tl @ ..] => {
                let mut contigs = Contigs::from(fst.index());
                for reader in tl.iter() {
                    contigs.add_index(reader.index());
                }

                let ids = vec![0; readers.len()];

                Self {
                    readers,
                    contigs,
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
    /// Note that the number of provided buffers and their contents must match the inner readers
    /// and their contents, respectively. See [`Self::create_record_bufs`] to create an appropriate
    /// collection of buffers based on the reader indices.
    pub fn read_records(&mut self, bufs: &mut [IdRecord]) -> io::Result<ReadStatus> {
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

    pub(crate) fn from_reader(reader: BgzfReader<R, V>) -> Self {
        Self {
            contigs: Contigs::from(reader.index()),
            readers: vec![reader],
            ids: vec![0],
        }
    }

    fn read_until_shared_contig(&mut self, bufs: &mut [IdRecord]) -> io::Result<ReadStatus> {
        let mut next_idx = 0;
        for (reader, buf) in self.readers.iter_mut().zip(bufs.iter_mut()) {
            match self.contigs.next_shared(*buf.contig_id(), reader.index()) {
                Some(idx) => {
                    if idx > next_idx {
                        next_idx = idx;
                    }
                }
                None => return Ok(ReadStatus::Done),
            }
        }

        let next_ids = &self.contigs[next_idx];
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

    fn read_until_shared_position_on_contig(
        &mut self,
        bufs: &mut [IdRecord],
    ) -> io::Result<Option<ReadStatus>> {
        let mut max_pos = bufs
            .iter()
            .map(IdRecord::position)
            .max()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty buffer slice"))?;

        'outer: loop {
            for ((reader, record), id) in self
                .readers
                .iter_mut()
                .zip(bufs.iter_mut())
                .zip(self.ids.iter_mut())
            {
                let mut pos = record.position();

                match pos.cmp(&max_pos) {
                    Ordering::Less => {
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

                        continue 'outer;
                    }
                    Ordering::Equal => (),
                    Ordering::Greater => {
                        max_pos = pos;
                        continue 'outer;
                    }
                }
            }

            return Ok(Some(ReadStatus::NotDone));
        }
    }
}

#[derive(Clone, Debug)]
struct Contigs {
    contigs: IndexMap<String, Vec<usize>>,
}

impl Contigs {
    fn add_index(&mut self, index: &Index) {
        let map: IndexMap<&str, usize> = index
            .records()
            .iter()
            .enumerate()
            .map(|(i, record)| (record.name(), i))
            .collect();

        self.contigs.retain(|name, ids| {
            if let Some(new_id) = map.get(name.as_str()) {
                ids.push(*new_id);
                true
            } else {
                false
            }
        })
    }

    fn next_shared(&self, id: usize, index: &Index) -> Option<usize> {
        let name = index.records()[id].name();

        self.contigs.get_index_of(name).or_else(|| {
            index.records()[(id + 1)..]
                .iter()
                .find_map(|record| self.contigs.get_index_of(record.name()))
        })
    }
}

impl From<&Index> for Contigs {
    fn from(index: &Index) -> Self {
        index
            .records()
            .iter()
            .enumerate()
            .map(|(i, record)| (record.name().to_owned(), vec![i]))
            .collect()
    }
}

impl FromIterator<(String, Vec<usize>)> for Contigs {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (String, Vec<usize>)>,
    {
        let contigs = iter.into_iter().map(|(s, a)| (s, a)).collect();

        Self { contigs }
    }
}

impl std::ops::Index<usize> for Contigs {
    type Output = [usize];

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.contigs[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::saf::tests::*;

    fn test_intersect<R>(mut intersect: Intersect<R, V3>, shared: &[(&str, u32)]) -> io::Result<()>
    where
        R: io::BufRead + io::Seek,
    {
        let mut bufs = intersect.create_record_bufs();

        for (expected_contig, expected_pos) in shared.iter() {
            intersect.read_records(&mut bufs)?;

            for (i, buf) in bufs.iter().enumerate() {
                let id = *buf.contig_id();
                let contig = intersect.get_readers()[i].index().records()[id].name();
                let pos = buf.position();

                assert_eq!((contig, pos), (*expected_contig, *expected_pos));
            }
        }

        assert!(intersect.read_records(&mut bufs)?.is_done());

        Ok(())
    }

    #[test]
    fn test_intersect_two() -> io::Result<()> {
        let left_reader = reader!(records![
            "chr2":4, "chr2":7, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]);

        let right_reader = reader!(records![
            "chr1":1, "chr2":7, "chr4":2, "chr4":3, "chr5":1, "chr7":9, "chr8":2, "chr9":1,
        ]);

        let intersect = left_reader.intersect(right_reader);
        let shared = vec![("chr2", 7), ("chr5", 1), ("chr7", 9)];

        test_intersect(intersect, &shared)
    }

    #[test]
    fn test_intersect_finishes_with_shared_end() -> io::Result<()> {
        let left_reader = reader!(records!("chr1":2 => [0.]));
        let right_reader = reader!(records!("chr1":2 => [0.]));

        let intersect = left_reader.intersect(right_reader);
        let shared = vec![("chr1", 2)];

        test_intersect(intersect, &shared)
    }

    #[test]
    fn test_intersect_three() -> io::Result<()> {
        let fst_reader = reader!(records![
            "chr2":4, "chr2":7, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]);

        let snd_reader = reader!(records![
            "chr2":4, "chr2":7, "chr7":9, "chr8":1, "chr9":1,
        ]);

        let thd_reader = reader!(records![
           "chr2":4, "chr2":8, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]);

        let intersect = fst_reader.intersect(snd_reader).intersect(thd_reader);
        let shared = vec![("chr2", 4u32), ("chr7", 9), ("chr8", 1)];

        test_intersect(intersect, &shared)
    }
}
