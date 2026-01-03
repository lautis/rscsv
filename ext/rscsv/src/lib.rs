use magnus::{
    block::yield_value, function, prelude::*, Error, RArray, RString, Ruby, Value,
};
use std::io::Read;

fn generate_lines(rows: Vec<Vec<String>>) -> Result<String, Error> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    for row in rows {
        wtr.write_record(&row)
            .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;
    }

    let inner = wtr
        .into_inner()
        .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;

    String::from_utf8(inner)
        .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))
}

fn record_to_ruby_array(record: &csv::ByteRecord) -> Result<RArray, Error> {
    let array = RArray::with_capacity(record.len());
    for column in record.iter() {
        let column_str = RString::from_slice(column);
        array.push(column_str)?;
    }
    Ok(array)
}

struct EnumeratorRead {
    enumerator: Value,
    buffer: Option<Vec<u8>>,
}

impl EnumeratorRead {
    fn new(enumerator: Value) -> Self {
        EnumeratorRead {
            enumerator,
            buffer: None,
        }
    }

    fn read_and_store_overflow(&mut self, buf: &mut [u8], value: &[u8]) -> std::io::Result<usize> {
        if value.len() > buf.len() {
            let (current, next) = value.split_at(buf.len());
            buf.copy_from_slice(current);
            self.buffer = Some(next.to_vec());
            Ok(current.len())
        } else {
            buf[..value.len()].copy_from_slice(value);
            self.buffer = None;
            Ok(value.len())
        }
    }

    fn read_from_external(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result: Result<String, Error> = self.enumerator.funcall("next", ());
        match result {
            Ok(string) => self.read_and_store_overflow(buf, string.as_bytes()),
            Err(_) => {
                // StopIteration or other exception - signal EOF
                Ok(0)
            }
        }
    }
}

impl Read for EnumeratorRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.buffer.take() {
            Some(ref inner) => self.read_and_store_overflow(buf, inner),
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

fn yield_csv(enumerator: Value) -> Result<(), Error> {
    let mut reader = csv_reader(EnumeratorRead::new(enumerator));
    let mut record = csv::ByteRecord::new();

    loop {
        let has_record = reader
            .read_byte_record(&mut record)
            .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;

        if !has_record {
            break;
        }

        let row_array = record_to_ruby_array(&record)?;
        let _: Value = yield_value(row_array)?;
    }

    Ok(())
}

fn parse_csv(data: String) -> Result<RArray, Error> {
    let mut reader = csv_reader(data.as_bytes());
    let result = RArray::new();

    for record in reader.records() {
        let record = record
            .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;

        let row = RArray::with_capacity(record.len());
        for field in record.iter() {
            row.push(RString::new(field))?;
        }
        result.push(row)?;
    }

    Ok(result)
}

fn generate_line(row: Vec<String>) -> Result<String, Error> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(&row)
        .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;

    let inner = wtr
        .into_inner()
        .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))?;

    String::from_utf8(inner)
        .map_err(|e| Error::new(magnus::exception::runtime_error(), e.to_string()))
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let reader_class = ruby.define_class("RscsvReader", ruby.class_object())?;
    reader_class.define_singleton_method("each_internal", function!(yield_csv, 1))?;
    reader_class.define_singleton_method("parse", function!(parse_csv, 1))?;

    let writer_class = ruby.define_class("RscsvWriter", ruby.class_object())?;
    writer_class.define_singleton_method("generate_line", function!(generate_line, 1))?;
    writer_class.define_singleton_method("generate_lines", function!(generate_lines, 1))?;

    Ok(())
}
