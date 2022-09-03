use std::{fmt, io};

use angsd_saf::{
    version::{Version, V3, V4},
    Intersect, Record,
};

pub mod utils;
use utils::{get_alleles_v3, get_alleles_v4, reader_from_records};

/// Returns record with the same contig id and position as `target` in `records`, if it exists.
fn find_intersection<'a, V>(
    target: &Record<&'static str, V::Item>,
    records: &'a [Record<&'static str, V::Item>],
) -> Option<&'a Record<&'static str, V::Item>>
where
    V: Version,
{
    records.iter().find(|record| {
        record.contig_id() == target.contig_id() && record.position() == target.position()
    })
}

/// Returns the intersecting records among the provided records.
///
/// Each inner slice here corresponds to one "reader".
fn brute_force_intersect<V>(
    all_records: &[&[Record<&'static str, V::Item>]],
) -> Vec<Vec<Record<&'static str, V::Item>>>
where
    V: Version,
    V::Item: Clone + fmt::Debug,
{
    let mut intersection = Vec::new();

    'outer: for target in all_records[0] {
        let mut entry = Vec::new();

        for records in all_records {
            if let Some(matching_record) = find_intersection::<V>(target, records) {
                entry.push(matching_record.clone());
            } else {
                entry.clear();
                continue 'outer;
            }
        }

        // If we get to this point, then each "reader" has a matching record, contained in `entry`,
        // and this is an intersection
        intersection.push(entry.clone());
        entry.clear()
    }

    intersection
}

/// Test that writing-then-reading the provided records with an intersecting reader provides the
/// same set of records as a brute-force intersection.
///
/// Each inner slice of records here corresponds to one "reader".
fn test_intersect<V>(
    all_alleles: &[usize],
    all_records: &[&[Record<&'static str, V::Item>]],
) -> io::Result<()>
where
    V: Version,
    V::Item: Clone + fmt::Debug + PartialEq,
{
    let mut intersect = all_alleles
        .iter()
        .zip(all_records)
        .map(|(&alleles, records)| reader_from_records::<V>(alleles, records))
        .collect::<io::Result<Vec<_>>>()
        .map(Intersect::new)?;

    let all_expected_records = brute_force_intersect::<V>(all_records);

    let mut bufs = intersect.create_record_bufs();
    for expected_records in all_expected_records {
        intersect.read_records(&mut bufs)?;
        let read_records = bufs
            .iter()
            .zip(intersect.get_readers())
            .map(|(buf, reader)| buf.clone().to_named(reader.index()))
            .collect::<Vec<_>>();

        assert_eq!(read_records, expected_records);
    }

    assert!(intersect.read_records(&mut bufs)?.is_done());

    Ok(())
}

fn test_intersect_v3(
    all_records: &[&[Record<&'static str, <V3 as Version>::Item>]],
) -> io::Result<()> {
    let all_alleles = all_records
        .iter()
        .map(|records| get_alleles_v3(records))
        .collect::<Vec<_>>();
    test_intersect::<V3>(&all_alleles, all_records)
}

fn test_intersect_v4(
    all_records: &[&[Record<&'static str, <V4 as Version>::Item>]],
) -> io::Result<()> {
    let all_alleles = all_records
        .iter()
        .map(|records| get_alleles_v4(records))
        .collect::<Vec<_>>();
    test_intersect::<V4>(&all_alleles, all_records)
}

#[test]
fn test_intersect_two_v3() -> io::Result<()> {
    test_intersect_v3(&[records_v3![c1:1], records_v3![c1:1]])?;

    test_intersect_v3(&[records_v3![c1:1, c1:2, c1:3], records_v3![c1:1, c1:2, c1:3]])?;

    test_intersect_v3(&[
        records_v3![c1:1, c1:2, c1:3,       c2:2],
        records_v3![c1:1, c1:2,       c2:1, c2:2],
    ])?;

    test_intersect_v3(&[
        records_v3![
                  c2:4, c2:7,             c5:1, c5:2, c7:9, c8:1,
        ],
        records_v3![
            c1:1,       c2:7, c4:2, c4:3, c5:1,       c7:9,       c8:2, c9:1,
        ],
    ])?;

    Ok(())
}

#[test]
fn test_intersect_two_v4() -> io::Result<()> {
    test_intersect_v4(&[records_v4![c1:1], records_v4![c1:2]])?;

    test_intersect_v4(&[records_v4![c1:1], records_v4![c2:1]])?;

    test_intersect_v4(&[records_v4![c1:1, c2:1, c3:1], records_v4![c1:1, c2:1, c3:1]])?;

    test_intersect_v4(&[
        records_v4![      c1:2,       c2:2],
        records_v4![c1:1, c1:2, c2:1, c2:2],
    ])?;

    test_intersect_v4(&[
        records_v4![
                  c2:4, c2:7, c5:2, c7:9, c8:1,
        ],
        records_v4![
            c1:1,       c2:7,       c7:9, c8:1,
        ],
    ])?;

    Ok(())
}

#[test]
fn test_intersect_three_v3() -> io::Result<()> {
    test_intersect_v3(&[records_v3![c1:1], records_v3![c1:1], records_v3![c1:1]])?;

    test_intersect_v3(&[
        records_v3![c1:1, c1:2, c1:3],
        records_v3![c1:1, c1:2, c1:3],
        records_v3![c1:1,       c1:3],
    ])?;

    test_intersect_v3(&[
        records_v3![
            c2:4, c2:7,       c5:1, c5:2, c7:9, c8:1,
        ],
        records_v3![
            c2:4, c2:7,                   c7:9, c8:1, c9:1,
        ],
        records_v3![
            c2:4,       c2:8, c5:1, c5:2, c7:9, c8:1,
        ],
    ])?;

    Ok(())
}

#[test]
fn test_intersect_three_v4() -> io::Result<()> {
    test_intersect_v4(&[records_v4![c1:1], records_v4![c2:1], records_v4![c1:1]])?;

    test_intersect_v4(&[
        records_v4![c1:1, c1:2, c1:3],
        records_v4![c1:1, c1:2, c1:3],
        records_v4![c1:1, c1:2, c1:3],
    ])?;

    test_intersect_v4(&[
        records_v4![
            c2:4, c2:7,       c5:1, c5:2, c7:9, c8:1,
        ],
        records_v4![
                  c2:7,                   c7:9, c8:1,
        ],
        records_v4![
            c2:4, c2:7, c2:8, c5:1, c5:2, c7:9, c8:1,
        ],
    ])?;

    Ok(())
}
