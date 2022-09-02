use std::io;

use angsd_saf::version::V3;

mod utils;

#[test]
fn test_write_read_index() -> io::Result<()> {
    let records = records!["chr1":1, "chr1":2, "chr2":1, "chr2":2, "chr2":3, "chr3":1];
    let reader = reader!(V3, records)?;

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
    let mut reader = reader!(V3, records)?;

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
    let mut reader = reader!(V3, records)?;

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
