//! Read text-based SAF records and print as SAF files.

use std::{
    env,
    io::{self, BufRead},
};

use angsd_io::saf;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("missing path to SAF member file");

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    let record: saf::Record<String, saf::record::Likelihoods> = buf.parse()?;

    let mut writer = saf::WriterV3::from_member_path(record.alleles(), path)?;
    writer.write_record(&record)?;

    while reader.read_line(&mut buf)? != 0 {
        buf.clear();
        let record: saf::Record<String, saf::record::Likelihoods> = buf.parse()?;
        writer.write_record(&record)?;
    }

    Ok(())
}
