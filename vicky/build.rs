use std::{fs, io};
use std::fs::File;
use std::io::{BufWriter, Write};

use regex::{Captures, Regex};

fn main() {
    if let Err(e) = ensure_locks_id_nullable() {
        eprintln!("Couldn't update schema.rs: {e}")
    }
}

const SCHEMA_RS_PATH: &str = "src/lib/database/schema.rs";

// tablename, fieldname, old type
// add more optional table values here as needed
const TABLE_FIELDS: [(&str, &str, &str); 1] = [("locks", "id", "Uuid")];

fn ensure_locks_id_nullable() -> io::Result<()> {
    println!("cargo::rerun-if-changed=src/lib/database/schema.rs");
    let mut new_schema = fs::read_to_string(SCHEMA_RS_PATH)?;
    for (table_name, field_name, old_type) in TABLE_FIELDS.iter() {
        let regex = Regex::new(&format!(
            r"(diesel::table!\s*\{{\n\s*{}\s*\({}\)\s*\{{\n\s*{}\s*->\s*){}",
            table_name, field_name, field_name, old_type,
        ))
        .expect("The values entered for the replacement were not valid.");

        new_schema = regex
            .replace(&new_schema, |caps: &Captures| {
                if let Some(before) = caps.get(1) {
                    format!("{}Nullable<{}>", before.as_str(), old_type)
                } else {
                    caps.get(0).unwrap().as_str().to_string() // shouldn't get here, but just in case
                }
            })
            .to_string();
    }

    let schema = File::create(SCHEMA_RS_PATH)?;
    let mut overwriter = BufWriter::new(schema);
    overwriter.write_all(new_schema.as_bytes())?;
    Ok(())
}
