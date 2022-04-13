//! Reading and writing of the SAF format.
//!
//! # Examples
//!
//! Read BGZF SAF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_read.rs"))]
//! ```
//!
//! Write BGZF SAF file:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_write.rs"))]
//! ```
//!
//! The above examples are also available as runnable binaries,
//! see the repository `examples/` folder.

use std::io;

pub mod index;
pub use index::Index;

pub mod reader;
pub use reader::{BgzfReader, Reader};

pub mod record;
pub use record::{IdRecord, Record};

pub mod writer;
pub use writer::{BgzfWriter, Writer};

pub mod ext {
    //! Conventional file name extensions for SAF files.

    use std::path::Path;

    /// Conventional index file extension.
    pub const INDEX_EXT: &str = "saf.idx";

    /// Conventional positions file extension.
    pub const POSITIONS_FILE_EXT: &str = "saf.pos.gz";

    /// Conventional values file extension.
    pub const VALUES_FILE_EXT: &str = "saf.gz";

    const EXTS: [&str; 3] = [INDEX_EXT, POSITIONS_FILE_EXT, VALUES_FILE_EXT];

    pub(super) fn prefix_from_member_path<P>(member_path: &P) -> Option<&str>
    where
        P: AsRef<Path>,
    {
        let s = member_path.as_ref().as_os_str().to_str()?;

        EXTS.into_iter()
            .find(|ext| s.ends_with(ext))
            .and_then(|ext| s.strip_suffix(ext))
            .and_then(|s_stem| s_stem.strip_suffix('.'))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_prefix_from_member_path() {
            assert_eq!(prefix_from_member_path(&"foo.saf.idx"), Some("foo"));
            assert_eq!(
                prefix_from_member_path(&"dir/bar.saf.pos.gz"),
                Some("dir/bar")
            );
            assert_eq!(
                prefix_from_member_path(&"/home/dir/baz.saf.gz"),
                Some("/home/dir/baz"),
            );
        }
    }
}

pub(self) const MAGIC_NUMBER: &[u8; 8] = &[b's', b'a', b'f', b'v', b'3', 0, 0, 0];

pub(self) type Endian = byteorder::LittleEndian;

pub(self) fn read_magic<R>(reader: &mut R) -> io::Result<()>
where
    R: io::Read + ?Sized,
{
    let mut magic = [0; 8];
    reader.read_exact(&mut magic)?;

    if &magic == MAGIC_NUMBER {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid or unsupported SAF magic number \
                (found '{magic:02x?}', expected '{MAGIC_NUMBER:02x?}')"
            ),
        ))
    }
}

pub(self) fn write_magic<W>(writer: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    writer.write_all(MAGIC_NUMBER)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Seek;

    use crate::saf::{
        reader::{BgzfPositionReader, BgzfValueReader, Intersect},
        writer::{BgzfPositionWriter, BgzfValueWriter},
    };

    type MockInner = io::Cursor<Vec<u8>>;
    type MockReader = BgzfReader<MockInner>;
    type MockWriter = BgzfWriter<MockInner, MockInner>;

    fn mock_writer(records: &[Record<&str>]) -> io::Result<MockWriter> {
        let mut index_writer = index::Writer::new(io::Cursor::new(Vec::new()));
        index_writer.write_magic()?;

        let mut position_writer = BgzfPositionWriter::from_bgzf(io::Cursor::new(Vec::new()));
        position_writer.write_magic()?;

        let mut value_writer = BgzfValueWriter::from_bgzf(io::Cursor::new(Vec::new()));
        value_writer.write_magic()?;

        let mut writer = MockWriter::new(index_writer, position_writer, value_writer);

        for record in records.iter() {
            writer.write_record(record)?;
        }

        Ok(writer)
    }

    fn mock_reader(writer: MockWriter) -> io::Result<Option<MockReader>> {
        let (mut index_cursor, mut position_cursor, mut value_cursor) = writer.finish()?;

        index_cursor.seek(io::SeekFrom::Start(0))?;
        position_cursor.seek(io::SeekFrom::Start(0))?;
        value_cursor.seek(io::SeekFrom::Start(0))?;

        let mut index_reader = index::Reader::new(index_cursor);
        let index = index_reader.read_index()?;

        let position_reader = BgzfPositionReader::from_bgzf(position_cursor);
        let value_reader = BgzfValueReader::from_bgzf(value_cursor);

        match MockReader::new(index, position_reader, value_reader) {
            Some(mut reader) => {
                reader.read_magic()?;

                Ok(Some(reader))
            }
            None => Ok(None),
        }
    }

    macro_rules! records {
        ($($contig:literal : $pos:literal => [$($v:literal),+ $(,)?]),+ $(,)?) => {
            vec![
                $(
                    Record::new(
                        $contig,
                        $pos,
                        Box::new([$($v),+]),
                    ),
                )+
            ]
        };
        (default) => {
            records!(
                "chr1":1 => [0., -1., -2., -3., -4.],
                "chr1":2 => [1., -2., -3., -4., -5.],
                "chr2":1 => [2., -3., -4., -5., -6.],
                "chr2":2 => [3., -4., -5., -6., -7.],
                "chr2":3 => [4., -5., -6., -7., -8.],
                "chr3":1 => [5., -6., -7., -8., -9.],
            )
        }
    }

    macro_rules! reader {
        ($records:expr) => {
            mock_reader(mock_writer($records.as_slice())?)?.unwrap()
        };
    }

    #[test]
    fn test_write_read_index() -> io::Result<()> {
        let reader = reader!(records!(default));

        let records = reader.index().records();

        assert_eq!(records.len(), 3);

        assert_eq!(records[0].name(), "chr1");
        assert_eq!(records[0].sites(), 2);

        assert_eq!(records[1].name(), "chr2");
        assert_eq!(records[1].sites(), 3);

        assert_eq!(records[2].name(), "chr3");
        assert_eq!(records[2].sites(), 1);

        Ok(())
    }

    #[test]
    fn test_write_read_records() -> io::Result<()> {
        let records = records!(default);
        let mut reader = reader!(records);

        let mut i = 0;
        let mut record = reader.create_record_buf();
        while reader.read_record(&mut record)?.is_not_done() {
            assert_eq!(record.values()[0], i as f32);

            i += 1;
        }

        assert_eq!(i, records.len());

        Ok(())
    }

    #[test]
    fn test_seek() -> io::Result<()> {
        let mut reader = reader!(records!(default));

        let mut record = reader.create_record_buf();

        for name in vec!["chr2", "chr1", "chr3"] {
            reader.seek_by_name(name)?;
            reader.read_record(&mut record)?;
            assert_eq!(record.position(), 1);

            let expected_name = reader.index().records()[*record.contig_id()].name();
            assert_eq!(expected_name, name);
        }

        Ok(())
    }

    fn test_intersect<R>(mut intersect: Intersect<R>, shared: &[(&str, u32)]) -> io::Result<()>
    where
        R: io::BufRead + io::Seek,
    {
        let mut bufs = intersect.create_record_bufs();

        for (expected_contig, expected_pos) in shared.iter() {
            intersect.read_records(&mut bufs)?;

            for (i, buf) in bufs.iter().enumerate() {
                let id = *buf.contig_id();
                let contig = intersect.get_readers()[i].index().records()[id].name();
                assert_eq!(contig, *expected_contig);

                let pos = buf.position();
                assert_eq!(pos, *expected_pos);
            }
        }

        assert!(intersect.read_records(&mut bufs)?.is_done());

        Ok(())
    }

    #[test]
    fn test_intersect_two() -> io::Result<()> {
        let left_reader = reader!(records![
            "chr2":4 => [0.],
            "chr2":7 => [0.],
            "chr5":1 => [0.],
            "chr5":2 => [0.],
            "chr7":9 => [0.],
            "chr8":1 => [0.],
        ]);

        let right_reader = reader!(records![
            "chr1":1 => [0.],
            "chr2":7 => [0.],
            "chr4":2 => [0.],
            "chr4":3 => [0.],
            "chr5":1 => [0.],
            "chr7":9 => [0.],
            "chr8":2 => [0.],
            "chr9":1 => [0.],
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
            "chr2":4 => [0.],
            "chr2":7 => [0.],
            "chr5":1 => [0.],
            "chr5":2 => [0.],
            "chr7":9 => [0.],
            "chr8":1 => [0.],
        ]);

        let snd_reader = reader!(records![
            "chr2":4 => [0.],
            "chr2":7 => [0.],
            "chr7":9 => [0.],
            "chr8":1 => [0.],
            "chr9":1 => [0.],
        ]);

        let thd_reader = reader!(records![
            "chr2":4 => [0.],
            "chr2":8 => [0.],
            "chr5":1 => [0.],
            "chr5":2 => [0.],
            "chr7":9 => [0.],
            "chr8":1 => [0.],
        ]);

        let intersect = fst_reader.intersect(snd_reader).intersect(thd_reader);
        let shared = vec![("chr2", 4u32), ("chr7", 9), ("chr8", 1)];

        test_intersect(intersect, &shared)
    }
}
