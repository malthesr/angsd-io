use std::{fs::File, io, marker::PhantomData, num::NonZeroUsize, path::Path};

use crate::{
    ext::{member_paths_from_prefix, prefix_from_member_path},
    version::{Version, V3, V4},
    Index,
};

use super::Reader;

/// A builder for a SAF reader.
#[derive(Debug)]
pub struct Builder<V> {
    threads: NonZeroUsize,
    v: PhantomData<V>,
}

type DefaultReader<V> = Reader<io::BufReader<File>, V>;

impl<V> Builder<V>
where
    V: Version,
{
    /// Builds a new reader from its components.
    ///
    /// The inner readers will be wrapped in [`bgzf::Reader`]s. The magic numbers will *not* be read
    /// so [`Reader::read_magic`] should be called manually before reading.
    ///
    /// Returns [`None`] if index contains no records.
    pub fn build<R>(
        self,
        index: Index<V>,
        position_reader: R,
        item_reader: R,
    ) -> Option<Reader<R, V>>
    where
        R: io::BufRead,
        V: Version,
    {
        Reader::from_bgzf(
            index,
            bgzf::reader::Builder::default()
                .set_worker_count(self.threads)
                .build_from_reader(position_reader),
            bgzf::reader::Builder::default()
                .set_worker_count(self.threads)
                .build_from_reader(item_reader),
        )
    }

    /// Builds a new reader from any member path.
    ///
    /// This method relies on stripping a conventional suffix from the member path and
    /// reconstructing all member paths. See [`Self::build_from_prefix`] for details on
    /// conventional naming.
    ///
    /// The magic numbers will be read, and so [`Reader::read_magic`] should *not* be called
    /// manually.
    pub fn build_from_member_path<P>(self, member_path: P) -> io::Result<DefaultReader<V>>
    where
        P: AsRef<Path>,
    {
        let s = member_path.as_ref().to_string_lossy();

        let prefix = prefix_from_member_path(&s).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Cannot determine shared SAF prefix from member path '{:?}'",
                    member_path.as_ref()
                ),
            )
        })?;

        self.build_from_prefix(prefix)
    }

    /// Builds a new reader from the paths of its components.
    ///
    /// The magic numbers will be read, and so [`Reader::read_magic`] should *not* be called
    /// manually.
    pub fn build_from_paths<P>(
        self,
        index_path: P,
        position_path: P,
        item_path: P,
    ) -> io::Result<DefaultReader<V>>
    where
        P: AsRef<Path>,
    {
        let index = Index::read_from_path(index_path)?;
        let position_reader = File::open(position_path).map(io::BufReader::new)?;
        let item_reader = File::open(item_path).map(io::BufReader::new)?;

        let mut new = self
            .build(index, position_reader, item_reader)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "empty index in reader construction",
                )
            })?;
        new.read_magic()?;
        Ok(new)
    }

    /// Builds a new reader from a shared prefix.
    ///
    /// Conventionally, the SAF index, positions, and item files are named according to a shared
    /// prefix and specific extensions for each file. See [`crate::ext`] for these extensions.
    /// Where this convention is observed, this method opens a reader from the shared prefix.
    ///
    /// The magic numbers will be read, and so [`Reader::read_magic`] should *not* be called
    /// manually.
    pub fn build_from_prefix<P>(self, prefix: P) -> io::Result<DefaultReader<V>>
    where
        P: AsRef<Path>,
    {
        let [index_path, position_path, item_path] =
            member_paths_from_prefix(&prefix.as_ref().to_string_lossy());

        self.build_from_paths(index_path, position_path, item_path)
    }

    /// Sets the number of threads to use in the reader.
    ///
    /// By default, the number of threads is 1.
    pub fn set_threads(mut self, threads: NonZeroUsize) -> Self {
        self.threads = threads;
        self
    }
}

impl Builder<V3> {
    /// Creates a builder for a new SAF V3 reader.
    pub fn v3() -> Self {
        Self::default()
    }
}

impl Builder<V4> {
    /// Creates a builder for a new SAF V4 reader.
    pub fn v4() -> Self {
        Self::default()
    }
}

impl<V> Default for Builder<V>
where
    V: Version,
{
    fn default() -> Self {
        Self {
            threads: NonZeroUsize::new(1).unwrap(),
            v: PhantomData,
        }
    }
}
