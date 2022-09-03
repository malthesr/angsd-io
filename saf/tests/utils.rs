use std::io::{self, Seek};

use angsd_saf::{
    record::{Band, Record},
    version::{Version, V3, V4},
    Index, Reader, Writer,
};

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

pub fn reader_from_writer<V>(writer: MockWriter<V>) -> io::Result<MockReader<V>>
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

pub fn reader_from_records<V>(
    alleles: usize,
    records: &[Record<&str, V::Item>],
) -> io::Result<MockReader<V>>
where
    V: Version,
{
    let mut writer = setup_writer(alleles)?;

    for record in records.iter() {
        writer.write_record(record)?;
    }

    reader_from_writer(writer)
}

/// Returns number of alleles for setup for V3 records.
pub fn get_alleles_v3<I>(records: &[Record<I, <V3 as Version>::Item>]) -> usize {
    records[0].alleles()
}

/// Returns number of alleles for setup for V4 records.
pub fn get_alleles_v4<I>(records: &[Record<I, <V4 as Version>::Item>]) -> usize {
    // The actual max is arbitrary for the purposes of testing: we simply pick the maximum seen
    records
        .iter()
        .map(|record| record.item().start() + record.item().likelihoods().len())
        .max()
        .unwrap_or(0)
}

#[macro_export]
macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

#[macro_export]
macro_rules! count {
    ($($tts:tt)*) => {<[()]>::len(&[$(replace_expr!($tts ())),*])};
}

#[macro_export]
macro_rules! records_v3 {
    ($($contig:ident : $pos:literal),+ $(,)?) => {
        &[$(records_v3!(@ $contig:$pos [0.])),+]
    };
    ($($contig:ident : $pos:literal [$($v:literal),+ $(,)?]),+ $(,)?) => {
        &[$(records_v3!(@ $contig:$pos [$($v),+])),+]
    };
    (@ $contig:ident : $pos:literal [$($v:literal),+ $(,)?]) => {
        ::angsd_saf::Record::new(
            stringify!($contig),
            $pos,
            vec![$($v),+].into(),
        )
    };
}

#[macro_export]
macro_rules! records_v4 {
    ($($contig:ident : $pos:literal),+ $(,)?) => {
        &[$(records_v4!(@ $contig:$pos [0.])),+]
    };
    ($($contig:ident : $pos:literal [$($($_t:ident),+ ;)? $($v:literal),+]),+ $(,)?) => {{
        &[$(records_v4!(@ $contig:$pos [$($($_t),+ ;)? $($v),+])),+ ]
    }};
    (@ $contig:ident : $pos:literal [ $($($_t:ident),+ ;)? $($v:literal),+ ]) => {
        ::angsd_saf::Record::new(
            stringify!($contig),
            $pos,
            ::angsd_saf::record::Band::new(
                count!($($($_t)+)?),
                vec![$($v),+],
            )
        )
    };
}

#[test]
fn test_records_v3_test_macro() {
    assert_eq!(
        records_v3!(chr1:1 [1., 2.]),
        &[Record::new("chr1", 1, vec![1., 2.])]
    );
    assert_eq!(
        records_v3!(
            chr7:100 [1., 2.],
            chr9:1 [0., -2.],
        ),
        &[
            Record::new("chr7", 100, vec![1., 2.]),
            Record::new("chr9", 1, vec![0., -2.]),
        ]
    );
    assert_eq!(
        records_v3!(chr1:1, chr1:2),
        &[
            Record::new("chr1", 1, vec![0.]),
            Record::new("chr1", 2, vec![0.]),
        ]
    );
}

#[test]
fn test_records_v4_test_macro() {
    assert_eq!(
        records_v4!(chr1:1 [1., 2.]),
        &[Record::new("chr1", 1, Band::new(0, vec![1., 2.]))]
    );
    assert_eq!(
        records_v4!(
            chr7:100 [nil; 1., 2.],
            chr8:2 [0.],
            chr10:1 [nil, nil, nil; 0., -2., -4., -8.],
        ),
        &[
            Record::new("chr7", 100, Band::new(1, vec![1., 2.])),
            Record::new("chr8", 2, Band::new(0, vec![0.])),
            Record::new("chr10", 1, Band::new(3, vec![0., -2., -4., -8.])),
        ]
    );
    assert_eq!(
        records_v4!(chr1:1, chr1:2),
        &[
            Record::new("chr1", 1, Band::new(0, vec![0.])),
            Record::new("chr1", 2, Band::new(0, vec![0.])),
        ]
    );
}
