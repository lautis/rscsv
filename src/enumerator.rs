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

extern fn protect_wrapper<F>(closure: *mut c_void) -> VALUE
      where F: FnOnce() -> VALUE {
    let closure_option = closure as *mut Option<F>;
    unsafe {
      (*closure_option).take().unwrap()()
    }
  }

fn protect<F>(func: F) -> Result<VALUE, RubyException>
where
    F: FnOnce() -> VALUE,
{
    let mut state = sys::EMPTY_EXCEPTION;
    let value = unsafe {
        sys::rb_protect(
            protect_wrapper::<F>,
            &func as *const _ as *mut c_void,
            &mut state,
        )
    };
    if state == sys::EMPTY_EXCEPTION {
        Ok(value)
    } else {
        Err(state)
    }
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

        let value = self.value;
        let result = protect(|| {
            unsafe { sys::rb_funcall(
                value,
                sys::rb_intern("next\0".as_ptr() as *const i8),
                0)
            }
        });
        match result {
            Ok(next) => {
                let string = String::from_ruby_unwrap(next);
                self.read_and_store_overflow(buf, string.as_bytes())
            },
            Err(state) => {
                unsafe { sys::rb_jump_tag(state) };
                //Err(std::io::Error::new(ErrorKind::Other, "Ruby Exception"))
            }
        }

    }
}

impl Read for EnumeratorRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.next.take() {
            Some(ref inner) => self.read_and_store_overflow(buf, inner),
            None => self.read_from_external(buf),
        }
    }
}

fn yield_csv(data: &Enumerator) -> Result<(), csv::Error> {
    let mut reader = csv_reader(EnumeratorRead::new(data.value));
    let mut record = csv::ByteRecord::new();

    while reader.read_byte_record(&mut record)? {
        let inner_array = record_to_ruby(&record);
        let result = protect(|| {
            unsafe {
                return sys::rb_yield(inner_array);
            }
        });

        if result.is_err() {
            unsafe { sys::rb_jump_tag(result.unwrap_err()) };
        }
    }

    Ok(())
}
