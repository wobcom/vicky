use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Write};
use std::io;

fn main() {
    if let Err(e) = ensure_locks_id_nullable() {
        eprintln!("Couldn't update schema.rs: {e}")
    }
}

fn ensure_locks_id_nullable() -> io::Result<()> {
    println!("cargo::rerun-if-changed=src/lib/database/schema.rs");
    let schema_rs_path = "src/lib/database/schema.rs";

    let schema = File::open(schema_rs_path)?;
    let reader = BufReader::new(schema);
    let mut new_schema: String = String::new();
    let mut found_locks_table = false;
    let mut only_copy = false;

    for line in reader.lines() {
        let line = line?;

        if only_copy {
            new_schema.push_str(&line);
        } else if !found_locks_table {
            if line.contains("locks (id) {") {
                found_locks_table = true;
            }
            new_schema.push_str(&line);
        } else {
            if !line.contains("id ->") {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "The line following the table header should've contained the id primary key",
                ));
            } else if line.contains("id -> Nullable<Int4>,") {
                return Ok(()); // already corrected
            }
            new_schema.push_str(&line.replace(" Int4,", " Nullable<Int4>,"));
            only_copy = true;
        }
        new_schema.push('\n');
    }
    let schema = File::create(schema_rs_path)?;
    let mut overwriter = BufWriter::new(schema);
    overwriter.write_all(new_schema.as_bytes())?;
    Ok(())
}
