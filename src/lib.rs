#[macro_use]
extern crate helix;
extern crate csv;

use std::error::Error;
use std::io::Read;
use helix::sys;
use helix::sys::{ID, VALUE};
use helix::{FromRuby, CheckResult, ToRuby, ToRubyResult};
use helix::libc::c_int;

#[cfg_attr(windows, link(name = "helix-runtime"))]
extern "C" {
    pub fn rb_block_given_p() -> c_int;
    pub fn rb_yield(value: VALUE);
    pub fn rb_funcall(value: VALUE, name: ID, nargs: c_int, ...) -> VALUE;
}

struct Records(Vec<csv::StringRecord>);

impl ToRuby for Records {
    fn to_ruby(self) -> ToRubyResult {
        let ary = unsafe { sys::rb_ary_new_capa(self.0.len() as isize) };
        for row in self.0 {
            let inner_array = unsafe { sys::rb_ary_new_capa(row.len() as isize) };
            for column in row.iter() {
                unsafe {
                    sys::rb_ary_push(inner_array, column.to_ruby()?);
                }
            }
            unsafe {
                sys::rb_ary_push(ary, inner_array);
            }
        }
        Ok(ary)
    }
}

fn generate_lines(rows: Vec<Vec<String>>) -> Result<String, Box<Error>> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    for row in rows {
        wtr.write_record(&row)?;
    }

    Ok(String::from_utf8(wtr.into_inner()?)?)
}

fn record_to_ruby(record: &csv::ByteRecord) -> VALUE {
    let inner_array = unsafe { sys::rb_ary_new_capa(record.len() as isize) };
    for column in record.iter() {
        unsafe {
            let column_value =
                sys::rb_utf8_str_new(column.as_ptr() as *const i8, column.len() as i64);
            sys::rb_ary_push(inner_array, column_value);
        }
    }
    inner_array
}

struct Enumerator {
    value: VALUE,
}

impl FromRuby for Enumerator {
    type Checked = Enumerator;

    fn from_ruby(value: VALUE) -> CheckResult<Enumerator> {
        // TODO: validate this?
        Ok(Enumerator { value })
    }

    fn from_checked(checked: Enumerator) -> Enumerator {
        checked
    }
}

struct EnumeratorRead {
    value: VALUE,
    next: Option<Vec<u8>>,
}

impl EnumeratorRead {
    fn new(value: VALUE) -> EnumeratorRead {
        EnumeratorRead {
            value,
            next: None,
        }
    }

    fn read_and_store_overflow(&mut self, buf: &mut [u8], value: &[u8]) -> std::io::Result<usize> {
        if value.len() > buf.len() {
            match value.split_at(buf.len()) {
                (current, next) => {
                    for (index, c) in current.iter().enumerate() {
                        buf[index] = *c;
                    }
                    self.next = Some(next.to_vec());
                    Ok(current.len())
                }
            }

        } else {
            for (index, value) in value.iter().enumerate() {
                buf[index] = *value;
            }
            self.next = None;
            Ok(value.len() as usize)
        }
    }

    fn read_from_external(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let next = unsafe {
            rb_funcall(
                self.value,
                sys::rb_intern("next\0".as_ptr() as *const i8),
                0,
            )
        };

        let string = String::from_ruby_unwrap(next);

        self.read_and_store_overflow(buf, string.as_bytes())
    }
}

impl Read for EnumeratorRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.next.clone() {
            Some(inner) => self.read_and_store_overflow(buf, &inner),
            None => self.read_from_external(buf),
        }
    }
}

fn csv_reader<R: Read>(reader: R) -> csv::Reader<R> {
    csv::ReaderBuilder::new()
        .buffer_capacity(16 * 1024)
        .has_headers(false)
        .from_reader(reader)
}

fn yield_csv(data: Enumerator) -> Result<(), csv::Error> {
    let mut reader = csv_reader(EnumeratorRead::new(data.value));
    let mut record = csv::ByteRecord::new();

    while reader.read_byte_record(&mut record)? {
        let inner_array = record_to_ruby(&record);
        unsafe {
            rb_yield(inner_array);
        }
    }

    Ok(())
}

fn parse_csv(data: String) -> Result<Vec<csv::StringRecord>, csv::Error> {
    csv_reader(data.as_bytes()).records().collect()
}

ruby! {
    class RscsvReader {
        def each_internal(data: Enumerator) -> () {
            match yield_csv(data) {
                Err(_) => raise_panic!("Error parsing CSV"),
                Ok(_) => ()
            }
        }

        def parse(data: String) -> Records {
            match parse_csv(data) {
                Err(_) => raise_panic!("Error parsing CSV"),
                Ok(result) => Records(result)
            }
        }
    }

    class RscsvWriter {
        def generate_line(row: Vec<String>) -> String {
            let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
            let result = wtr.write_record(&row);
            match result {
                Err(_) => raise_panic!("Error generating csv"),
                Ok(_) => String::from_utf8(wtr.into_inner().unwrap()).unwrap(),
            }
        }

        def generate_lines(rows: Vec<Vec<String>>) -> String {
            match generate_lines(rows) {
                Err(_) => raise_panic!("Error generating csv"),
                Ok(csv) => csv
            }
        }
    }
}
