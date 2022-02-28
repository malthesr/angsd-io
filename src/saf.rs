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

    fn mock_records() -> Vec<Record<&'static str>> {
        vec![
            Record::new("chr1", 1, Box::new([0., -1., -2., -3., -4.])),
            Record::new("chr1", 2, Box::new([1., -2., -3., -4., -5.])),
            Record::new("chr2", 1, Box::new([2., -3., -4., -5., -6.])),
            Record::new("chr2", 2, Box::new([3., -4., -5., -6., -7.])),
            Record::new("chr2", 3, Box::new([4., -5., -6., -7., -8.])),
            Record::new("chr3", 1, Box::new([5., -6., -7., -8., -9.])),
        ]
    }

    #[test]
    fn test_write_read_index() -> io::Result<()> {
        let reader = mock_reader(mock_writer(&mock_records())?)?.unwrap();

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
        let records = mock_records();
        let mut reader = mock_reader(mock_writer(&records)?)?.unwrap();

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
        let mut reader = mock_reader(mock_writer(&mock_records())?)?.unwrap();

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

    #[test]
    fn test_intersect() -> io::Result<()> {
        let left_records = vec![
            Record::new("chr2", 4, Box::new([0.])),
            Record::new("chr2", 7, Box::new([0.])),
            Record::new("chr5", 1, Box::new([0.])),
            Record::new("chr5", 2, Box::new([0.])),
            Record::new("chr7", 9, Box::new([0.])),
            Record::new("chr8", 1, Box::new([0.])),
        ];
        let left_reader = mock_reader(mock_writer(&left_records)?)?.unwrap();

        let right_records = vec![
            Record::new("chr1", 1, Box::new([0.])),
            Record::new("chr2", 7, Box::new([0.])),
            Record::new("chr4", 2, Box::new([0.])),
            Record::new("chr4", 3, Box::new([0.])),
            Record::new("chr5", 1, Box::new([0.])),
            Record::new("chr7", 9, Box::new([0.])),
            Record::new("chr8", 2, Box::new([0.])),
            Record::new("chr9", 1, Box::new([0.])),
        ];
        let right_reader = mock_reader(mock_writer(&right_records)?)?.unwrap();

        let mut intersect = Intersect::new(left_reader, right_reader);
        let (mut left, mut right) = intersect.create_record_buf();

        let shared = vec![("chr2", 7u32), ("chr5", 1), ("chr7", 9)];

        for (contig, pos) in shared.into_iter() {
            println!("{contig}:{pos}");
            intersect.read_record_pair(&mut left, &mut right)?;

            let left_contig = intersect.get_left().index().records()[*left.contig_id()].name();
            assert_eq!(left_contig, contig);
            assert_eq!(left.position(), pos);

            let right_contig = intersect.get_right().index().records()[*right.contig_id()].name();
            assert_eq!(right_contig, contig);
            assert_eq!(right.position(), pos);
        }

        assert!(intersect.read_record_pair(&mut left, &mut right)?.is_done());

        Ok(())
    }

    #[test]
    fn test_intersect_finishes_with_shared_end() -> io::Result<()> {
        let left_reader =
            mock_reader(mock_writer(&[Record::new("chr1", 2, Box::new([0.]))])?)?.unwrap();
        let right_reader =
            mock_reader(mock_writer(&[Record::new("chr1", 2, Box::new([0.]))])?)?.unwrap();
        let mut intersect = Intersect::new(left_reader, right_reader);

        let (mut left, mut right) = intersect.create_record_buf();
        intersect.read_record_pair(&mut left, &mut right)?;

        assert_eq!(*left.contig_id(), 0);
        assert_eq!(left.position(), 2);
        assert_eq!(*right.contig_id(), 0);
        assert_eq!(right.position(), 2);

        assert!(intersect.read_record_pair(&mut left, &mut right)?.is_done());

        Ok(())
    }
}
