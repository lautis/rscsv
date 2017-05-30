#[macro_use]
extern crate helix;
extern crate csv;

use std::error::Error;
use std::io::Read;
use std::slice::from_raw_parts;
use helix::sys;
use helix::sys::{VALUE, ID};
use helix::{UncheckedValue, CheckResult, CheckedValue, ToRust, ToRuby};
use helix::libc::c_int;

struct VecWrap<T>(Vec<T>);

impl<T> UncheckedValue<VecWrap<T>> for VALUE
    where VALUE: UncheckedValue<T>
{
    fn to_checked(self) -> CheckResult<VecWrap<T>> {
        if unsafe { sys::RB_TYPE_P(self, sys::T_ARRAY) } {
            let slice = ruby_array_to_slice(self);
            for val in slice.iter() {
                if let Err(error) = val.to_checked() {
                    return Err(format!("Failed to convert value for Vec<T>: {}", error));
                }
            }
            Ok(unsafe { CheckedValue::<VecWrap<T>>::new(self) })
        } else {
            let val = unsafe { CheckedValue::<String>::new(sys::rb_inspect(self)) };
            Err(format!("No implicit conversion of {} into Vec<String>",
                        val.to_rust()))
        }
    }
}

fn ruby_array_to_slice<'a>(array: VALUE) -> &'a [VALUE] {
    let length = unsafe { sys::RARRAY_LEN(array) } as usize;
    unsafe { from_raw_parts(sys::RARRAY_CONST_PTR(array), length) }
}

impl ToRust<VecWrap<String>> for CheckedValue<VecWrap<String>>
    where VALUE: UncheckedValue<String>,
          CheckedValue<String>: ToRust<String>
{
    fn to_rust(self) -> VecWrap<String> {
        let slice = ruby_array_to_slice(self.inner);
        let mut vec: Vec<String> = Vec::with_capacity(slice.len());
        for val in slice.iter() {
            let checked = val.to_checked().unwrap();
            vec.push(checked.to_rust());
        }
        VecWrap(vec)
    }
}

impl ToRust<VecWrap<VecWrap<String>>> for CheckedValue<VecWrap<VecWrap<String>>>
    where VALUE: UncheckedValue<VecWrap<String>>,
          CheckedValue<VecWrap<String>>: ToRust<VecWrap<String>>
{
    fn to_rust(self) -> VecWrap<VecWrap<String>> {
        let slice = ruby_array_to_slice(self.inner);
        let mut vec: Vec<VecWrap<String>> = Vec::with_capacity(slice.len());
        for val in slice.iter() {
            let checked = val.to_checked().unwrap();
            vec.push(checked.to_rust());
        }
        VecWrap(vec)
    }
}

#[cfg_attr(windows, link(name="helix-runtime"))]
extern "C" {
    pub fn rb_ary_new_capa(capa: isize) -> VALUE;
    pub fn rb_ary_entry(ary: VALUE, offset: isize) -> VALUE;
    pub fn rb_ary_push(ary: VALUE, item: VALUE) -> VALUE;
    pub fn rb_block_given_p() -> c_int;
    pub fn rb_yield(value: VALUE);
    pub fn rb_funcall(value: VALUE, name: ID, nargs: c_int, ...) -> VALUE;
}

impl ToRuby for VecWrap<csv::StringRecord> {
    fn to_ruby(self) -> VALUE {
        let ary = unsafe { rb_ary_new_capa(self.0.len() as isize) };
        for row in self.0 {
            let inner_array = unsafe { rb_ary_new_capa(row.len() as isize) };
            for column in row.iter() {
                unsafe {
                    rb_ary_push(inner_array, column.to_ruby());
                }
            }
            unsafe {
                rb_ary_push(ary, inner_array);
            }
        }
        ary
    }
}

fn generate_lines(rows: VecWrap<VecWrap<String>>) -> Result<String, Box<Error>> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    for row in rows.0 {
        wtr.write_record(&(row.0))?;
    }

    Ok(String::from_utf8(wtr.into_inner()?)?)
}

fn record_to_ruby(record: &csv::ByteRecord) -> VALUE {
    let inner_array = unsafe { rb_ary_new_capa(record.len() as isize) };
    for column in record.iter() {
        unsafe {
            let column_value = sys::rb_utf8_str_new(column.as_ptr() as *const i8,
                                                    column.len() as i64);
            rb_ary_push(inner_array, column_value);
        }
    }
    inner_array
}


impl UncheckedValue<Enumerator> for VALUE {
    fn to_checked(self) -> CheckResult<Enumerator> {
        Ok(unsafe { CheckedValue::new(self) })
    }
}

impl ToRust<Enumerator> for CheckedValue<Enumerator> {
    fn to_rust(self) -> Enumerator {
        Enumerator { value: self.inner }
    }
}

struct Enumerator {
    value: VALUE,
}

struct EnumeratorRead {
    value: VALUE,
    next: Option<Vec<u8>>,
}

impl EnumeratorRead {
    fn new(value: VALUE) -> EnumeratorRead {
        EnumeratorRead {
            value: value,
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
            rb_funcall(self.value,
                       sys::rb_intern("next\0".as_ptr() as *const i8),
                       0)
        };
        let slice = unsafe {
            from_raw_parts(sys::RSTRING_PTR(next) as *const u8,
                           sys::RSTRING_LEN(next) as usize)
        };

        self.read_and_store_overflow(buf, slice)
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
    let mut reader = csv_reader(data.as_bytes());
    reader
        .records()
        .collect::<Result<Vec<csv::StringRecord>, csv::Error>>()
}

ruby! {
    class RscsvReader {
        def each_internal(data: Enumerator) {
            match yield_csv(data) {
                Err(_) => throw!("Error parsing CSV"),
                Ok(_) => ()
            }
        }
        def parse(data: String) -> VecWrap<csv::StringRecord> {
            match parse_csv(data) {
                Err(_) => throw!("Error parsing CSV"),
                Ok(result) => VecWrap(result)
            }
        }
    }
    class RscsvWriter {
        def generate_line(row: VecWrap<String>) -> String {
            let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
            let result = wtr.write_record(&(row.0));
            match result {
                Err(_) => throw!("Error generating csv"),
                Ok(_) => String::from_utf8(wtr.into_inner().unwrap()).unwrap(),
            }
        }

        def generate_lines(rows: VecWrap<VecWrap<String>>) -> String {
            match generate_lines(rows) {
                Err(_) => throw!("Error generating csv"),
                Ok(csv) => csv
            }
        }
    }
}
