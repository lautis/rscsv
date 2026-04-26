use magnus::{
    encoding::{EncodingCapable, Index},
    function,
    prelude::*,
    value::ReprValue,
    Error, RArray, RString, Ruby, Value,
};
use std::io::{self, Read, Write};

#[inline]
fn type_error(ruby: &Ruby, msg: &'static str) -> Error {
    Error::new(ruby.exception_type_error(), msg)
}

#[inline]
fn runtime_error<E: ToString>(ruby: &Ruby, e: E) -> Error {
    Error::new(ruby.exception_runtime_error(), e.to_string())
}

// ============================================================================
// Writer
// ============================================================================

/// `io::Write` adapter that appends bytes directly into a Ruby `RString`'s
/// buffer via `rb_str_cat`. This skips the intermediate `Vec<u8>` -> `RString`
/// copy that we'd otherwise pay at the end of a write call.
///
/// # Safety / GC notes
/// The held `RString` is on the Rust stack (inside the `csv::Writer`), so MRI's
/// conservative stack scan keeps it reachable. `rb_str_cat` may allocate (and
/// thus trigger GC), but our `buf` slice points into Rust-owned memory (the
/// csv writer's internal buffer), not Ruby-managed memory.
struct RStringWriter {
    rstring: RString,
}

impl Write for RStringWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.rstring.cat(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Push each cell of `row` into `record` (copying bytes into the record buffer).
fn fill_record(ruby: &Ruby, row: RArray, record: &mut csv::ByteRecord) -> Result<(), Error> {
    record.clear();
    // Safety: `row` is held alive by the Ruby caller's reference. We don't
    // call back into Ruby code that could trigger GC between obtaining the
    // slice and copying field bytes into `record`.
    let cells = unsafe { row.as_slice() };
    for &cell_value in cells {
        let s = RString::from_value(cell_value)
            .ok_or_else(|| type_error(ruby, "expected String values inside row"))?;
        // Safety: `s` is reachable via the row RArray while we copy its bytes
        // into `record` (which performs an internal copy).
        record.push_field(unsafe { s.as_slice() });
    }
    Ok(())
}

#[inline]
fn write_buffer_capacity(rows: usize) -> usize {
    // Heuristic upper bound for typical workloads (a generous 256B/row).
    // Used to pre-allocate the destination RString so `rb_str_cat` doesn't
    // need to grow it during writes.
    rows.saturating_mul(256).max(4096)
}

fn new_utf8_rstring(ruby: &Ruby, capacity: usize) -> RString {
    let s = ruby.str_buf_new(capacity);
    let _ = s.enc_associate(ruby.utf8_encindex());
    s
}

fn generate_lines(ruby: &Ruby, rows: RArray) -> Result<RString, Error> {
    let row_count = rows.len();
    let rstring = new_utf8_rstring(ruby, write_buffer_capacity(row_count));
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(64 * 1024)
        .from_writer(RStringWriter { rstring });

    let mut record = csv::ByteRecord::new();

    // Safety: `rows` is the function argument; the Ruby caller holds a
    // reference to it for the duration of this call, so its element values
    // remain reachable while we iterate.
    let row_values = unsafe { rows.as_slice() };
    for &row_value in row_values {
        let row = RArray::from_value(row_value)
            .ok_or_else(|| type_error(ruby, "expected Array of Arrays"))?;
        fill_record(ruby, row, &mut record)?;
        wtr.write_byte_record(&record)
            .map_err(|e| runtime_error(ruby, e))?;
    }

    Ok(wtr.into_inner().map_err(|e| runtime_error(ruby, e))?.rstring)
}

fn generate_line(ruby: &Ruby, row: RArray) -> Result<RString, Error> {
    // For the single-row path, output is typically small (< embedded-string
    // limit). Going via an intermediate `Vec<u8>` and one `enc_str_new` lets
    // MRI pick an embedded RString and skips the `rb_str_cat`-driven growth
    // path that the multi-row writer benefits from.
    let mut wtr = csv::WriterBuilder::new()
        .buffer_capacity(1024)
        .from_writer(Vec::<u8>::with_capacity(256));
    let mut record = csv::ByteRecord::new();
    fill_record(ruby, row, &mut record)?;
    wtr.write_byte_record(&record)
        .map_err(|e| runtime_error(ruby, e))?;
    let inner = wtr.into_inner().map_err(|e| runtime_error(ruby, e))?;
    Ok(ruby.enc_str_new(&inner, ruby.utf8_encoding()))
}

// ============================================================================
// Reader (parse_csv) — direct csv-core path, bypasses csv crate's BufReader
// ============================================================================

const STACK_CAP: usize = 64;
const ROW_BATCH: usize = 256;
const INITIAL_FIELD_BYTES: usize = 4096;
const INITIAL_ENDS: usize = 64;

/// Build a Ruby Array from `output[..outlen]` and `ends[..endlen]`, tagging
/// each resulting field with the supplied encoding so that callers see strings
/// in the input's encoding rather than always-BINARY.
#[inline]
fn build_row_from_buffers(
    ruby: &Ruby,
    output: &[u8],
    ends: &[usize],
    encoding: Index,
) -> Result<RArray, Error> {
    let len = ends.len();
    if len <= STACK_CAP {
        let qnil = ruby.qnil().as_value();
        let mut buf = [qnil; STACK_CAP];
        let mut start = 0usize;
        for (i, &end) in ends.iter().enumerate() {
            // Safety: csv-core guarantees `end <= output.len()` and ends is non-decreasing.
            let field = unsafe { output.get_unchecked(start..end) };
            buf[i] = ruby.enc_str_new(field, encoding).as_value();
            start = end;
        }
        Ok(ruby.ary_new_from_values(&buf[..len]))
    } else {
        let array = ruby.ary_new_capa(len);
        let mut start = 0usize;
        for &end in ends {
            let field = unsafe { output.get_unchecked(start..end) };
            array.push(ruby.enc_str_new(field, encoding))?;
            start = end;
        }
        Ok(array)
    }
}

fn parse_csv(ruby: &Ruby, data: RString) -> Result<RArray, Error> {
    // Preserve the input string's encoding on every produced field rather
    // than always handing back BINARY-tagged strings.
    let encoding: Index = data.enc_get();

    // Safety: `data` is the function argument and is held alive by the caller
    // throughout this call. We never trigger Ruby code that could mutate or
    // free its buffer while we hold the slice.
    let mut input: &[u8] = unsafe { data.as_slice() };

    let mut core = csv_core::ReaderBuilder::new().build();

    // Estimate row count from input size to pre-size the result array.
    let estimated_rows = (input.len() / 360).max(8);
    let result = ruby.ary_new_capa(estimated_rows);

    // Per-record byte/ends scratch buffers — grown on demand.
    let mut output = vec![0u8; INITIAL_FIELD_BYTES];
    let mut ends = vec![0usize; INITIAL_ENDS];

    // Batch row RArrays in a stack buffer. Stack VALUES are conservatively
    // scanned by MRI's GC, so previously created row arrays remain alive
    // across allocations.
    let qnil = ruby.qnil().as_value();
    let mut batch: [Value; ROW_BATCH] = [qnil; ROW_BATCH];
    let mut batch_len = 0usize;

    let mut outlen = 0usize;
    let mut endlen = 0usize;

    'records: loop {
        let (res, nin, nout, nend) =
            core.read_record(input, &mut output[outlen..], &mut ends[endlen..]);
        input = &input[nin..];
        outlen += nout;
        endlen += nend;

        match res {
            csv_core::ReadRecordResult::InputEmpty => {
                if input.is_empty() {
                    // Signal EOF by passing empty input on the next iteration;
                    // csv-core will emit any remaining record then End.
                    continue 'records;
                }
                // Should not happen when input is non-empty, but loop again to be safe.
                continue 'records;
            }
            csv_core::ReadRecordResult::OutputFull => {
                let new_len = (output.len() * 2).max(outlen + 1);
                output.resize(new_len, 0);
                continue 'records;
            }
            csv_core::ReadRecordResult::OutputEndsFull => {
                let new_len = (ends.len() * 2).max(endlen + 1);
                ends.resize(new_len, 0);
                continue 'records;
            }
            csv_core::ReadRecordResult::Record => {
                let row =
                    build_row_from_buffers(ruby, &output[..outlen], &ends[..endlen], encoding)?;
                batch[batch_len] = row.as_value();
                batch_len += 1;
                if batch_len == ROW_BATCH {
                    result.cat(&batch)?;
                    batch_len = 0;
                }
                outlen = 0;
                endlen = 0;
                continue 'records;
            }
            csv_core::ReadRecordResult::End => {
                break 'records;
            }
        }
    }

    if batch_len > 0 {
        result.cat(&batch[..batch_len])?;
    }

    Ok(result)
}

// ============================================================================
// Reader (each) — uses csv crate Reader since input arrives as Ruby chunks
// ============================================================================

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
        let result: Result<Value, Error> = self.enumerator.funcall("next", ());
        match result {
            Ok(value) => match RString::from_value(value) {
                Some(rstring) => {
                    // Safety: We immediately copy the bytes out before yielding
                    // control back to Ruby, so GC can't invalidate them in between.
                    let bytes = unsafe { rstring.as_slice() };
                    self.read_and_store_overflow(buf, bytes)
                }
                None => Ok(0),
            },
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

fn record_to_ruby_array(ruby: &Ruby, record: &csv::ByteRecord) -> Result<RArray, Error> {
    let len = record.len();

    if len <= STACK_CAP {
        let qnil = ruby.qnil().as_value();
        let mut buf = [qnil; STACK_CAP];
        for (i, column) in record.iter().enumerate() {
            buf[i] = ruby.str_from_slice(column).as_value();
        }
        Ok(ruby.ary_new_from_values(&buf[..len]))
    } else {
        let array = ruby.ary_new_capa(len);
        for column in record.iter() {
            array.push(ruby.str_from_slice(column))?;
        }
        Ok(array)
    }
}

fn yield_csv(ruby: &Ruby, enumerator: Value) -> Result<(), Error> {
    let mut reader = csv::ReaderBuilder::new()
        .buffer_capacity(64 * 1024)
        .has_headers(false)
        .from_reader(EnumeratorRead::new(enumerator));
    let mut record = csv::ByteRecord::new();

    loop {
        let has_record = reader
            .read_byte_record(&mut record)
            .map_err(|e| runtime_error(ruby, e))?;

        if !has_record {
            break;
        }

        let row_array = record_to_ruby_array(ruby, &record)?;
        let _: Value = ruby.yield_value(row_array)?;
    }

    Ok(())
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
