//! Read intersecting sites in two SAF files and print readable sites.

use std::{
    env,
    io::{self, Write},
};

use angsd_io::saf;

fn main() -> io::Result<()> {
    let readers = env::args()
        .skip(1)
        .map(|p| saf::ReaderV3::from_member_path(p))
        .collect::<io::Result<Vec<_>>>()?;

    // Note also the [`Reader::intersect`] and [`Intersect::intersect`] methods to construct
    // intersecting reader when the number of readers are statically known.
    let mut intersect = saf::Intersect::new(readers);

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut bufs = intersect.create_record_bufs();
    while intersect.read_records(&mut bufs)?.is_not_done() {
        for (reader, buf) in intersect.get_readers().iter().zip(bufs.iter()) {
            let contig = reader.index().records()[*buf.contig_id()].name();
            let position = buf.position();
            write!(writer, "{contig}\t{position}")?;

            for v in buf.item() {
                write!(writer, "\t{v:.2}")?;
            }

            writeln!(writer)?;
        }
    }

    Ok(())
}
