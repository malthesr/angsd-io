//! Read intersecting sites in two BGZF SAF files and print readable sites.

use std::{
    env,
    io::{self, Write},
};

use angsd_io::saf;

fn main() -> io::Result<()> {
    let readers = env::args()
        .skip(1)
        .map(|p| saf::BgzfReader::<_, saf::V3>::from_bgzf_member_path(p))
        .collect::<io::Result<Vec<_>>>()?;

    // Note also the [`BgzfReader::intersect`] and [`Intersect::intersect`] methods to construct
    // intersecting reader when the number of readers are statically known.
    let mut intersect = saf::reader::Intersect::new(readers);

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut bufs = intersect.create_record_bufs();
    while intersect.read_records(&mut bufs)?.is_not_done() {
        for (reader, buf) in intersect.get_readers().iter().zip(bufs.iter()) {
            let contig = reader.index().records()[*buf.contig_id()].name();
            let position = buf.position();
            write!(writer, "{contig}\t{position}")?;

            for value in buf.contents() {
                write!(writer, "\t{value:.2}")?;
            }

            writeln!(writer)?;
        }
    }

    Ok(())
}
