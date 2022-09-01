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
pub use reader::{Reader, ReaderV3, ReaderV4};

pub mod record;
pub use record::Record;

mod version;
pub use version::{Version, V3, V4};

pub mod writer;
pub use writer::{BgzfWriter, Writer};

#[cfg(test)]
pub(self) mod tests {
    use super::*;

    use std::io::{self, Seek};

    pub type MockReader = Reader<io::Cursor<Vec<u8>>, V3>;
    pub type MockBgzfWriter = BgzfWriter<io::Cursor<Vec<u8>>, io::Cursor<Vec<u8>>, V3>;

    impl MockBgzfWriter {
        pub fn create() -> Self {
            let index_writer = io::Cursor::new(Vec::new());
            let position_writer = bgzf::Writer::new(io::Cursor::new(Vec::new()));
            let item_writer = bgzf::Writer::new(io::Cursor::new(Vec::new()));

            let mut new = Self::new(index_writer, position_writer, item_writer);
            new.write_magic().unwrap();
            new
        }
    }

    impl From<MockBgzfWriter> for MockReader {
        fn from(writer: MockBgzfWriter) -> Self {
            let (mut index_reader, mut position_reader, mut item_reader) = writer.finish().unwrap();

            index_reader.seek(io::SeekFrom::Start(0)).unwrap();
            position_reader.seek(io::SeekFrom::Start(0)).unwrap();
            item_reader.seek(io::SeekFrom::Start(0)).unwrap();

            let index = Index::read(&mut index_reader).unwrap();

            let mut new = Self::new(index, position_reader, item_reader).unwrap();
            new.read_magic().unwrap();
            new
        }
    }

    macro_rules! reader {
        ($records:expr) => {{
            let mut writer = MockBgzfWriter::create();

            for record in $records.iter() {
                writer.write_record(record).unwrap();
            }

            MockReader::from(writer)
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
            assert_eq!(record.item()[0], i as f32);

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
