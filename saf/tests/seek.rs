use std::io;

use angsd_saf::version::V3;

pub mod utils;
use utils::reader_from_records;

#[test]
fn test_seek_v3() -> io::Result<()> {
    let records = records_v3![chr1:1, chr1:2, chr2:1, chr2:2, chr2:3, chr3:1];
    let mut reader = reader_from_records::<V3>(0, records)?;

    let mut record = reader.create_record_buf();

    for name in &["chr2", "chr1", "chr3"] {
        reader.seek_by_name(name)?;
        reader.read_record(&mut record)?;
        assert_eq!(record.position(), 1);

        let expected_name = reader.index().records()[*record.contig_id()].name();
        assert_eq!(&expected_name, name);
    }

    Ok(())
}
