//! Read a SAF file and print readable index and records.

use std::{
    env,
    io::{self, Write},
};

use angsd_saf as saf;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("missing path to SAF member file");
    let mut reader = saf::ReaderV3::from_member_path(path)?;

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    write!(writer, "{}", reader.index())?;

    let mut record = reader.create_record_buf();
    while reader.read_record(&mut record)?.is_not_done() {
        writeln!(writer, "{record:.2}")?;
    }

    Ok(())
}
