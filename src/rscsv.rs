#[macro_use]
extern crate helix;
extern crate csv;

use helix::sys;
use helix::sys::VALUE;
use helix::{UncheckedValue, CheckResult, CheckedValue, ToRust};

struct VecWrap<T>(Vec<T>);

impl<T> UncheckedValue<VecWrap<T>> for VALUE
    where VALUE: UncheckedValue<T>
{
    fn to_checked(self) -> CheckResult<VecWrap<T>> {
        if unsafe { sys::RB_TYPE_P(self, sys::T_ARRAY) } {
            // TODO: Make sure we can actually do the conversions for the values.
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
        let mut vec: Vec<String> = Vec::new();
        let len = unsafe { sys::RARRAY_LEN(self.inner) };
        let ptr = unsafe { sys::RARRAY_PTR(self.inner) };
        for i in 0..len {
            let val = unsafe { *ptr.offset(i) };
            let checked = val.to_checked().unwrap();
            vec.push(checked.to_rust());
        }
        return VecWrap(vec);
    }
}

ruby! {
    class RscsvWriter {
        def generate_line(row: VecWrap<String>) -> String {
            let mut writer = csv::Writer::from_memory();
            writer.write(row.0.into_iter()).unwrap();
            let result = writer.as_string();
            return result.to_owned();
        }
    }
}
