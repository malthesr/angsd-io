use std::io::{self, Seek};

use angsd_saf::{version::Version, Index, Reader, Writer};

pub type MockReader<V> = Reader<io::Cursor<Vec<u8>>, V>;
pub type MockWriter<V> = Writer<io::Cursor<Vec<u8>>, V>;

pub fn setup_writer<V>(alleles: usize) -> io::Result<MockWriter<V>>
where
    V: Version,
{
    let mut new = Writer::new(
        io::Cursor::new(Vec::new()),
        io::Cursor::new(Vec::new()),
        io::Cursor::new(Vec::new()),
    );
    new.write_magic()?;
    new.write_alleles(alleles)?;
    Ok(new)
}

pub fn setup_reader_from_writer<V>(writer: MockWriter<V>) -> io::Result<MockReader<V>>
where
    V: Version,
{
    let (mut index_reader, mut position_reader, mut item_reader) = writer.finish()?;

    index_reader.seek(io::SeekFrom::Start(0))?;
    position_reader.seek(io::SeekFrom::Start(0))?;
    item_reader.seek(io::SeekFrom::Start(0))?;

    let index = Index::read(&mut index_reader)?;

    let mut new = Reader::new(index, position_reader, item_reader)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty index"))?;
    new.read_magic()?;
    Ok(new)
}

#[macro_export]
macro_rules! reader {
    ($v:ident, $records:expr) => {{
        let mut writer = crate::utils::setup_writer($records[0].alleles())?;

        for record in $records.iter() {
            writer.write_record(record)?;
        }

        crate::utils::setup_reader_from_writer::<$v>(writer)
    }};
}

#[macro_export]
macro_rules! records {
    ($($contig:literal : $pos:literal => [$($v:literal),+ $(,)?]),+ $(,)?) => {
        vec![
            $(
                ::angsd_saf::Record::new(
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
                ::angsd_saf::Record::new(
                    $contig,
                    $pos,
                    vec![0.],
                ),
            )+
        ]
    };
}
