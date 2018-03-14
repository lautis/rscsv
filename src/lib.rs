#[macro_use]
extern crate helix;
extern crate csv;

use std::error::Error;
use std::io::Read;
use helix::sys;
use helix::sys::{VALUE, RubyException};
use helix::{FromRuby, CheckResult, ToRuby};
use helix::libc::{c_void};

include!("enumerator.rs");

fn generate_lines(rows: &[Vec<String>]) -> Result<String, Box<Error>> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    for row in rows {
        wtr.write_record(row)?;
    }

    Ok(String::from_utf8(wtr.into_inner()?)?)
}

fn csv_reader<R: Read>(reader: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new()
        .buffer_capacity(16 * 1024)
        .has_headers(false)
        .from_reader(reader)
}

fn parse_csv(data: &str) -> Result<Vec<Vec<VALUE>>, csv::Error> {
    csv_reader(data.as_bytes())
        .records()
        .map(|r| r.map(|v| record_to_vec(&v)))
        .collect()
}

fn record_to_vec(record: &csv::StringRecord) -> Vec<VALUE> {
    record.iter().map(|s| s.to_ruby().unwrap()).collect()
}

ruby! {
    class RscsvReader {
        def each_internal(data: Enumerator) -> Result<(), &'static str> {
            yield_csv(&data).map_err(|_| "Error parsing CSV")
        }

        def parse(data: String) -> Result<Vec<Vec<VALUE>>, &'static str> {
            parse_csv(&data).map_err(|_| "Error parsing CSV")
        }
    }

    class RscsvWriter {
        def generate_line(row: Vec<String>) -> Result<String, &'static str> {
            let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);

            wtr.write_record(&row)
                .map(|_| String::from_utf8(wtr.into_inner().unwrap()).unwrap())
                .map_err(|_| "Error generating csv")
        }

        def generate_lines(rows: Vec<Vec<String>>) -> Result<String, &'static str> {
            generate_lines(&rows).map_err(|_| "Error generating csv")
        }
    }
}
