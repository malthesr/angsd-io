//! Read intersecting sites in two BGZF SAF file and contig and position.

use std::{
    env,
    io::{self, Write},
};

use angsd_io::saf;

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let left_path = args.next().expect("missing path to first SAF member file");
    let right_path = args.next().expect("missing path to second SAF member file");

    let left_reader = saf::BgzfReader::from_bgzf_member_path(left_path)?;
    let right_reader = saf::BgzfReader::from_bgzf_member_path(right_path)?;
    let mut reader = left_reader.intersect(right_reader);

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut bufs = reader.create_record_bufs();
    while reader.read_records(&mut bufs)?.is_not_done() {
        let left_id = *bufs[0].contig_id();
        let left_contig = reader.get_readers()[0].index().records()[left_id].name();
        let left_pos = bufs[0].position();

        let right_id = *bufs[1].contig_id();
        let right_contig = reader.get_readers()[1].index().records()[right_id].name();
        let right_pos = bufs[1].position();

        assert_eq!(left_contig, right_contig);
        assert_eq!(left_pos, right_pos);

        writeln!(writer, "{left_contig}:{left_pos}")?;
    }

    Ok(())
}
