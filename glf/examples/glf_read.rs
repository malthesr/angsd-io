//! Read a BGZF GLF file and print readable records.

use std::{
    env,
    io::{self, Write},
};

use angsd_glf as glf;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).expect("missing path to GLF file");
    let n = env::args()
        .nth(2)
        .map(|s| s.parse().expect("failed to parse number of individuals"))
        .unwrap_or(1);
    let mut reader = glf::BgzfReader::from_bgzf_path(path)?;

    let stdout = io::stdout();
    let mut writer = stdout.lock();

    let mut records = vec![glf::Record::default(); n];
    while reader.read_records(records.as_mut_slice())?.is_not_done() {
        write!(writer, "{:.2}", records[0])?;

        for record in records.iter().skip(1) {
            write!(writer, "\t{:.2}", record)?;
        }

        writeln!(writer)?;
    }

    Ok(())
}
