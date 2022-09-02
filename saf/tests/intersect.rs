use std::io;

mod utils;

use angsd_saf::{
    version::{Version, V3},
    Intersect,
};

fn test_intersect<R, V>(mut intersect: Intersect<R, V>, shared: &[(&str, u32)]) -> io::Result<()>
where
    R: io::BufRead + io::Seek,
    V: Version,
{
    let mut bufs = intersect.create_record_bufs();

    for (expected_contig, expected_pos) in shared.iter() {
        intersect.read_records(&mut bufs)?;

        for (i, buf) in bufs.iter().enumerate() {
            let id = *buf.contig_id();
            let contig = intersect.get_readers()[i].index().records()[id].name();
            let pos = buf.position();

            assert_eq!((contig, pos), (*expected_contig, *expected_pos));
        }
    }

    assert!(intersect.read_records(&mut bufs)?.is_done());

    Ok(())
}

#[test]
fn test_intersect_two() -> io::Result<()> {
    let left_reader = reader!(
        V3,
        records![
            "chr2":4, "chr2":7, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]
    )?;

    let right_reader = reader!(
        V3,
        records![
            "chr1":1, "chr2":7, "chr4":2, "chr4":3, "chr5":1, "chr7":9, "chr8":2, "chr9":1,
        ]
    )?;

    let intersect = left_reader.intersect(right_reader);
    let shared = vec![("chr2", 7), ("chr5", 1), ("chr7", 9)];

    test_intersect(intersect, &shared)
}

#[test]
fn test_intersect_two_simple() -> io::Result<()> {
    let left_reader = reader!(
        V3,
        records![
            "chr1":1, "chr1":2, "chr1":3
        ]
    )?;

    let right_reader = reader!(
        V3,
        records![
            "chr1":1, "chr1":2, "chr1":3
        ]
    )?;

    let intersect = left_reader.intersect(right_reader);
    let shared = vec![("chr1", 1), ("chr1", 2), ("chr1", 3)];

    test_intersect(intersect, &shared)
}

#[test]
fn test_intersect_finishes_with_shared_end() -> io::Result<()> {
    let left_reader = reader!(V3, records!("chr1":2 => [0.]))?;
    let right_reader = reader!(V3, records!("chr1":2 => [0.]))?;

    let intersect = left_reader.intersect(right_reader);
    let shared = vec![("chr1", 2)];

    test_intersect(intersect, &shared)
}

#[test]
fn test_intersect_three() -> io::Result<()> {
    let fst_reader = reader!(
        V3,
        records![
            "chr2":4, "chr2":7, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]
    )?;

    let snd_reader = reader!(
        V3,
        records![
            "chr2":4, "chr2":7, "chr7":9, "chr8":1, "chr9":1,
        ]
    )?;

    let thd_reader = reader!(
        V3,
        records![
           "chr2":4, "chr2":8, "chr5":1, "chr5":2, "chr7":9, "chr8":1,
        ]
    )?;

    let intersect = fst_reader.intersect(snd_reader).intersect(thd_reader);
    let shared = vec![("chr2", 4u32), ("chr7", 9), ("chr8", 1)];

    test_intersect(intersect, &shared)
}
