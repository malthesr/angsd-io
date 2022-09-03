use std::{fmt, io};

use angsd_saf::{
    version::{Version, V3, V4},
    Index, Record,
};

pub mod utils;
use utils::{get_alleles_v3, get_alleles_v4, reader_from_records, MockReader};

/// Test that contigs names and sites per contig index matches those in provided records.
fn test_index_matches_records<V>(index: &Index<V>, records: &[Record<&str, V::Item>])
where
    V: Version,
{
    let (contigs, sites) = index
        .records()
        .iter()
        .map(|x| (x.name(), x.sites()))
        .unzip::<_, _, Vec<_>, Vec<_>>();

    let mut record_contigs = records
        .iter()
        .map(|record| record.contig_id().to_string())
        .collect::<Vec<_>>();
    record_contigs.dedup();
    assert_eq!(contigs, record_contigs);

    let record_sites = record_contigs
        .iter()
        .map(|contig| {
            records
                .iter()
                .filter(|record| record.contig_id() == contig)
                .count()
        })
        .collect::<Vec<_>>();
    assert_eq!(sites, record_sites);
}

/// Test that records in reader matches provided records.
fn test_reader_matches_records<V>(
    reader: &mut MockReader<V>,
    records: &[Record<&str, V::Item>],
) -> io::Result<()>
where
    V: Version,
    V::Item: Clone + fmt::Debug + PartialEq,
{
    let mut buf = reader.create_record_buf();

    for expected_record in records {
        reader.read_record(&mut buf)?;
        let read_record = buf.clone().to_named(reader.index());

        assert_eq!(&read_record, expected_record);
    }

    assert!(reader.read_record(&mut buf)?.is_done());

    Ok(())
}

/// Tests that index and records in reader matches provided records after writing-and-reading.
fn test_write_read<V>(alleles: usize, records: &[Record<&str, V::Item>]) -> io::Result<()>
where
    V: Version,
    V::Item: Clone + fmt::Debug + PartialEq,
{
    let mut reader = reader_from_records::<V>(alleles, records)?;

    assert_eq!(reader.index().alleles(), alleles);
    test_index_matches_records(reader.index(), records);
    test_reader_matches_records(&mut reader, records)
}

fn test_write_read_v3(records: &[Record<&str, <V3 as Version>::Item>]) -> io::Result<()> {
    test_write_read::<V3>(get_alleles_v3(records), records)
}

fn test_write_read_v4(records: &[Record<&str, <V4 as Version>::Item>]) -> io::Result<()> {
    test_write_read::<V4>(get_alleles_v4(records), records)
}

#[test]
fn test_v3_single_record() -> io::Result<()> {
    test_write_read_v3(records_v3![
        chr1:1 [0.],
    ])?;
    test_write_read_v3(records_v3![
        chr1:1 [0., -1.],
    ])?;
    test_write_read_v3(records_v3![
        chr1:1 [-0.5, 0., -1.],
    ])?;

    Ok(())
}

#[test]
fn test_v4_single_record() -> io::Result<()> {
    test_write_read_v4(records_v4![
        chr1:1 [-0.]
    ])?;
    test_write_read_v4(records_v4![
        chr1:1 [nil; -0., -1.]
    ])?;
    test_write_read_v4(records_v4![
        chr1:1 [nil, nil, nil; -0.5, -0., -1.]
    ])?;

    Ok(())
}

#[test]
fn test_v3_single_contig() -> io::Result<()> {
    test_write_read_v3(records_v3![
        chr1:1 [0., -1., -5.],
        chr1:2 [-1., 0., -2.],
    ])?;
    test_write_read_v3(records_v3![
        chr1:1 [0., -1.],
        chr1:2 [-0.1, 0.],
        chr1:4 [-0.001, 0.],
        chr1:5 [0., -1.],
        chr1:10 [-10., 0.],
    ])?;

    Ok(())
}

#[test]
fn test_v4_single_contig() -> io::Result<()> {
    test_write_read_v4(records_v4![
        chr1:1 [-1., 0.],
        chr1:2 [nil, nil; 0., -1., -5.],
    ])?;
    test_write_read_v4(records_v4![
        chr1:1 [nil; 0., -1.],
        chr1:2 [-0.1, 0., -0.2],
        chr1:4 [nil, nil, nil; 0.],
        chr1:5 [nil, nil; 0., -1.],
        chr1:10 [-10., 0., -4.],
    ])?;

    Ok(())
}

#[test]
fn test_v3_two_contigs() -> io::Result<()> {
    test_write_read_v3(records_v3![
        chr1:1 [0., -1., -2., -4., -8.],
        chr1:4 [-4., -2., 0., -1., -2.],
        chr2:2 [-8., -8., -2., -1., 0.],
        chr2:20 [-0.5, -0.25, -0.125, 0., -0.05],
    ])
}

#[test]
fn test_v4_two_contigs() -> io::Result<()> {
    test_write_read_v4(records_v4![
        chr1:1 [nil, nil, nil, nil; 0., -1., -2., -4., -8.],
        chr1:4 [nil; -2., 0.],
        chr2:2 [-8., -8., -2., -1., 0.],
        chr2:20 [nil, nil; 0., -0.05],
    ])
}

#[test]
fn test_v3_single_record_many_contigs() -> io::Result<()> {
    test_write_read_v3(records_v3![
        chr1:1 [0.],
        chr2:3 [-1.],
        chr4:10 [-2.],
        chr5:5 [-3.],
        chr10:1000 [-4.],
    ])
}

#[test]
fn test_v4_single_record_many_contigs() -> io::Result<()> {
    test_write_read_v4(records_v4![
        chr1:1 [nil; 0.],
        chr2:3 [-1.],
        chr4:10 [nil, nil, nil;-2.],
        chr5:5 [nil; -3.],
        chr10:1000 [nil, nil; -4.],
    ])
}
