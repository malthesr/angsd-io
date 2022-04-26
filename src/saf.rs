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
//! Read only intersecting sites in multiple BGZF SAF files:
//!
//! ```no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/saf_intersect.rs"))]
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

/// SAF file magic number.
pub const MAGIC_NUMBER: &[u8; 8] = &[b's', b'a', b'f', b'v', b'3', 0, 0, 0];

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
pub(self) mod tests {
    use super::*;

    use std::io::Seek;

    use crate::saf::{
        reader::{BgzfPositionReader, BgzfValueReader},
        writer::{BgzfPositionWriter, BgzfValueWriter},
    };

    pub type MockBgzfReader = BgzfReader<io::Cursor<Vec<u8>>>;
    pub type MockBgzfWriter = BgzfWriter<io::Cursor<Vec<u8>>, io::Cursor<Vec<u8>>>;

    impl MockBgzfWriter {
        pub fn create() -> Self {
            let mut index_writer = index::Writer::new(io::Cursor::new(Vec::new()));
            index_writer.write_magic().unwrap();

            let mut position_writer = BgzfPositionWriter::from_bgzf(io::Cursor::new(Vec::new()));
            position_writer.write_magic().unwrap();

            let mut value_writer = BgzfValueWriter::from_bgzf(io::Cursor::new(Vec::new()));
            value_writer.write_magic().unwrap();

            Self::new(index_writer, position_writer, value_writer)
        }
    }

    impl From<MockBgzfWriter> for MockBgzfReader {
        fn from(writer: MockBgzfWriter) -> Self {
            let (mut index_cursor, mut position_cursor, mut value_cursor) =
                writer.finish().unwrap();

            index_cursor.seek(io::SeekFrom::Start(0)).unwrap();
            position_cursor.seek(io::SeekFrom::Start(0)).unwrap();
            value_cursor.seek(io::SeekFrom::Start(0)).unwrap();

            let mut index_reader = index::Reader::new(index_cursor);
            let index = index_reader.read_index().unwrap();

            let mut position_reader = BgzfPositionReader::from_bgzf(position_cursor);
            position_reader.read_magic().unwrap();

            let mut value_reader = BgzfValueReader::from_bgzf(value_cursor);
            value_reader.read_magic().unwrap();

            Self::new(index, position_reader, value_reader).unwrap()
        }
    }

    macro_rules! reader {
        ($records:expr) => {{
            let mut writer = MockBgzfWriter::create();

            for record in $records.iter() {
                writer.write_record(record).unwrap();
            }

            MockBgzfReader::from(writer)
        }};
    }
    pub(super) use reader;

    macro_rules! records {
        ($($contig:literal : $pos:literal => [$($v:literal),+ $(,)?]),+ $(,)?) => {
            vec![
                $(
                    crate::saf::Record::new(
                        $contig,
                        $pos,
                        Box::new([$($v),+]),
                    ),
                )+
            ]
        };
        ($($contig:literal : $pos:literal),+ $(,)?) => {
            vec![
                $(
                    crate::saf::Record::new(
                        $contig,
                        $pos,
                        Box::new([0.]),
                    ),
                )+
            ]
        };
    }
    pub(super) use records;

    #[test]
    fn test_write_read_index() -> io::Result<()> {
        let records = records!["chr1":1, "chr1":2, "chr2":1, "chr2":2, "chr2":3, "chr3":1];
        let reader = reader!(records);

        let index_records = reader.index().records();

        assert_eq!(index_records.len(), 3);

        assert_eq!(index_records[0].name(), "chr1");
        assert_eq!(index_records[0].sites(), 2);

        assert_eq!(index_records[1].name(), "chr2");
        assert_eq!(index_records[1].sites(), 3);

        assert_eq!(index_records[2].name(), "chr3");
        assert_eq!(index_records[2].sites(), 1);

        Ok(())
    }

    #[test]
    fn test_write_read_records() -> io::Result<()> {
        let records = records![
            "chr1":1 => [0., -1., -2., -3., -4.],
            "chr1":2 => [1., -2., -3., -4., -5.],
            "chr2":1 => [2., -3., -4., -5., -6.],
            "chr2":2 => [3., -4., -5., -6., -7.],
            "chr2":3 => [4., -5., -6., -7., -8.],
            "chr3":1 => [5., -6., -7., -8., -9.],
        ];
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
        let records = records!["chr1":1, "chr1":2, "chr2":1, "chr2":2, "chr2":3, "chr3":1];
        let mut reader = reader!(records);

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
}
