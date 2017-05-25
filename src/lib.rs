#[macro_use]
extern crate helix;
extern crate csv;

use std::error::Error;
use helix::sys;
use helix::sys::VALUE;
use helix::{UncheckedValue, CheckResult, CheckedValue, ToRust};

struct VecWrap<T>(Vec<T>);

impl<T> UncheckedValue<VecWrap<T>> for VALUE
    where VALUE: UncheckedValue<T>
{
    fn to_checked(self) -> CheckResult<VecWrap<T>> {
        if unsafe { sys::RB_TYPE_P(self, sys::T_ARRAY) } {
            let len = unsafe { sys::RARRAY_LEN(self) };
            let ptr = unsafe { sys::RARRAY_PTR(self) };
            for i in 0..len {
                let val = unsafe { *ptr.offset(i) };
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

impl ToRust<VecWrap<String>> for CheckedValue<VecWrap<String>>
    where VALUE: UncheckedValue<String>,
          CheckedValue<String>: ToRust<String>
{
    fn to_rust(self) -> VecWrap<String> {
        let len = unsafe { sys::RARRAY_LEN(self.inner) };
        let ptr = unsafe { sys::RARRAY_PTR(self.inner) };
        let mut vec: Vec<String> = Vec::with_capacity(len as usize);
        for i in 0..len {
            let val = unsafe { *ptr.offset(i) };
            let checked = val.to_checked().unwrap();
            vec.push(checked.to_rust());
        }
        return VecWrap(vec);
    }
}

impl ToRust<VecWrap<VecWrap<String>>> for CheckedValue<VecWrap<VecWrap<String>>>
    where VALUE: UncheckedValue<VecWrap<String>>,
          CheckedValue<VecWrap<String>>: ToRust<VecWrap<String>>
{
    fn to_rust(self) -> VecWrap<VecWrap<String>> {
        let len = unsafe { sys::RARRAY_LEN(self.inner) };
        let ptr = unsafe { sys::RARRAY_PTR(self.inner) };
        let mut vec: Vec<VecWrap<String>> = Vec::with_capacity(len as usize);
        for i in 0..len {
            let val = unsafe { *ptr.offset(i) };
            let checked = val.to_checked().unwrap();
            vec.push(checked.to_rust());
        }
        return VecWrap(vec);
    }
}

#[cfg_attr(windows, link(name="helix-runtime"))]
extern "C" {
    pub fn rb_ary_new_capa(capa: isize) -> VALUE;
    pub fn rb_ary_entry(ary: VALUE, offset: isize) -> VALUE;
    pub fn rb_ary_push(ary: VALUE, item: VALUE) -> VALUE;
}

fn generate_lines(rows: VecWrap<VecWrap<String>>) -> Result<String, Box<Error>> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    for row in rows.0 {
        wtr.write_record(&(row.0))?;
    }

    return Ok(String::from_utf8(wtr.into_inner()?)?);
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
    return inner_array;
}

fn parse_csv(data: &[u8]) -> Result<VALUE, csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(data);

    let value = unsafe { rb_ary_new_capa(0) };
    let mut record = csv::ByteRecord::new();

    while reader.read_byte_record(&mut record)? {
        let inner_array = record_to_ruby(&record);
        unsafe {
            rb_ary_push(value, inner_array);
        }
    }
    return Ok(value);
}

ruby! {
    class RscsvReader {
        def parse(data: String) -> VALUE {
            match parse_csv(data.as_bytes()) {
                Err(_) => throw!("Error parsing CSV"),
                Ok(result) => return result
            };
        }
    }
    class RscsvWriter {
        def generate_line(row: VecWrap<String>) -> String {
            let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
            let result = wtr.write_record(&(row.0));
            match result {
                Err(_) => throw!("Error generating csv"),
                Ok(_) => return String::from_utf8(wtr.into_inner().unwrap()).unwrap(),
            };
        }

        def generate_lines(rows: VecWrap<VecWrap<String>>) -> String {
            match generate_lines(rows) {
                Err(_) => throw!("Error generating csv"),
                Ok(csv) => csv
            }
        }
    }
}
