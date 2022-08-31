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

pub mod ext;

pub mod index;
pub use index::Index;

pub mod reader;
pub use reader::{BgzfReader, Reader};

pub mod record;
pub use record::Record;

mod version;
pub use version::{Version, V3};

pub mod writer;
pub use writer::{BgzfWriter, Writer};

pub(self) type Endian = byteorder::LittleEndian;

#[cfg(test)]
pub(self) mod tests {
    use super::*;

    use std::io::{self, Seek};

    use crate::saf::writer::{BgzfPositionWriter, BgzfValueWriter};

    pub type MockBgzfReader = BgzfReader<io::Cursor<Vec<u8>>, V3>;
    pub type MockBgzfWriter = BgzfWriter<io::Cursor<Vec<u8>>, io::Cursor<Vec<u8>>, V3>;

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

            let mut index_reader = index::Reader::<_, V3>::new(index_cursor);
            let index = index_reader.read_index().unwrap();

            let mut position_reader = bgzf::Reader::new(position_cursor);
            V3::read_magic(&mut position_reader).unwrap();

            let mut value_reader = bgzf::Reader::new(value_cursor);
            V3::read_magic(&mut value_reader).unwrap();

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
                        vec![$($v),+],
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
                        vec![0.],
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
            assert_eq!(record.contents()[0], i as f32);

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
