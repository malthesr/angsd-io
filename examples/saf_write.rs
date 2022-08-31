//! Read text-based SAF records and print as BGZF SAF files.

use std::{
    env,
    io::{self, BufRead},
};

use angsd_io::saf;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("missing path to SAF member file");
    let mut writer = saf::BgzfWriter::<_, _, saf::V3>::from_bgzf_member_path(path)?;

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let mut buf = String::new();
    while reader.read_line(&mut buf)? != 0 {
        let record: saf::Record<String, saf::record::Likelihoods> = buf.parse()?;
        writer.write_record(&record)?;
        buf.clear();
    }

    Ok(())
}
