//! ABI v1. All handles passed to these functions must obey runtime.h ownership.
use crate::native_value::NativeValue;
use ocelot_base::shared_string::SharedString;
use ocelot_pal::runtime_io;
use std::cell::RefCell;
use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::sync::Arc;

thread_local! {
    static REPORT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static EVALUATED: RefCell<bool> = const { RefCell::new(false) };
}

fn write_report(kind: &str, text: &str) {
    REPORT.with(|path| {
        if let Some(path) = path.borrow().as_ref()
            && runtime_io::report(path, &format!("ocelot-{kind}-v1\n{text}")).is_err()
        {
            runtime_io::exit(102);
        }
    });
}

fn fail(text: &str) -> ! {
    write_report("error", text);
    let _ = runtime_io::stderr(&format!("{text}\n"));
    runtime_io::exit(101)
}

/// Initializes ABI v1 and an optional private report path.
/// # Safety
/// A non-null path must point to a valid NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_init_v1(path: *const c_char) {
    if !path.is_null() {
        let path = unsafe { CStr::from_ptr(path) }
            .to_str()
            .expect("UTF-8 report path");
        REPORT.with(|report| *report.borrow_mut() = Some(PathBuf::from(path)));
    }
}

/// Creates an owned string from exact UTF-8 bytes.
/// # Safety
/// Bytes must identify a readable UTF-8 buffer of the specified length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_string(bytes: *const u8, len: usize) -> NativeValue {
    let text = unsafe { std::slice::from_raw_parts(bytes, len) };
    NativeValue::string(std::str::from_utf8(text).expect("generated UTF-8"))
}

/// Retains a borrowed value, producing an owned value.
/// # Safety
/// A string value must be a live runtime handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_keep(value: NativeValue) -> NativeValue {
    if value.tag == 2 {
        unsafe { Arc::increment_strong_count(value.payload as *const SharedString) };
    }
    value
}

/// Releases one owned handle.
/// # Safety
/// A string must own a reference not previously released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_release(value: NativeValue) {
    if value.tag == 2 {
        unsafe { Arc::decrement_strong_count(value.payload as *const SharedString) };
    }
}

/// Compares borrowed values, including mixed-type native assertions.
/// # Safety
/// String values must be live runtime handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_equal(left: NativeValue, right: NativeValue) -> i32 {
    i32::from(
        left.tag == right.tag
            && if left.tag == 2 {
                unsafe { left.text() == right.text() }
            } else {
                left.payload == right.payload
            },
    )
}

/// Concatenates borrowed values using display rendering.
/// # Safety
/// String values must be live runtime handles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_concat(left: NativeValue, right: NativeValue) -> NativeValue {
    NativeValue::string(format!("{}{}", unsafe { left.render(false) }, unsafe {
        right.render(false)
    }))
}

/// Prints a borrowed boolean or string.
/// # Safety
/// A string must be a live runtime handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_print(value: NativeValue) -> NativeValue {
    if value.tag == 0 {
        fail("type error: `println` expects a string or bool argument");
    }
    if let Err(error) = runtime_io::stdout(&format!("{}\n", unsafe { value.render(false) })) {
        fail(&error.kind().to_string());
    }
    NativeValue { tag: 0, payload: 0 }
}

/// Checks a boolean, reporting a pre-rendered source diagnostic on failure.
/// # Safety
/// Diagnostic must be a live string handle; condition must be boolean.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_assert(condition: NativeValue, diagnostic: NativeValue) -> NativeValue {
    if condition.payload == 0 {
        fail(unsafe { diagnostic.text() });
    }
    NativeValue { tag: 0, payload: 0 }
}

/// Checks equality and reports the same diff as the interpreter.
/// # Safety
/// All string values must be live handles; diagnostic must be a string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_assert_eq(
    expected: NativeValue,
    actual: NativeValue,
    diagnostic: NativeValue,
) -> NativeValue {
    if unsafe { oc_equal(expected, actual) } == 0 {
        fail(&format!(
            "{}\n\nexpected: {}\nactual:   {}",
            unsafe { diagnostic.text() },
            unsafe { expected.render(true) },
            unsafe { actual.render(true) }
        ));
    }
    NativeValue { tag: 0, payload: 0 }
}

/// Writes a typed eval result separately from program output.
/// # Safety
/// A string must be a live runtime handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oc_eval(value: NativeValue) {
    write_report("eval", &unsafe { value.render(true) });
    EVALUATED.with(|evaluated| *evaluated.borrow_mut() = true);
}

/// Finishes successful execution without overwriting a scenario's last result.
#[unsafe(no_mangle)]
pub extern "C" fn oc_finish() {
    EVALUATED.with(|evaluated| {
        if !*evaluated.borrow() {
            write_report("ok", "");
        }
    });
}
