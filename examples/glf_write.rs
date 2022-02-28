//! Read text-based GLF records and print as BGZF GLF file.

use std::{
    env,
    io::{self, BufRead},
};

use angsd_io::glf;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("missing path to GLF file");
    let mut writer = glf::BgzfWriter::from_bgzf_path(path)?;

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let mut buf = String::new();
    let mut records = Vec::new();

    while reader.read_line(&mut buf)? != 0 {
        for record in buf.split_whitespace() {
            records.push(record.parse()?);
        }
        writer.write_records(records.as_slice())?;

        buf.clear();
        records.clear();
    }

    Ok(())
}
