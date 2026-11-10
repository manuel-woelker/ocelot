use ocelot_base::shared_string::SharedString;
use std::sync::Arc;

/// Unboxed primitive or owned string handle passed by value across the C ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeValue {
    /// 0 = unit, 1 = boolean, 2 = string.
    pub tag: u64,
    /// Boolean bits or a pointer to an Arc-owned SharedString.
    pub payload: u64,
}

impl NativeValue {
    /// Constructs an owned string handle.
    pub fn string(text: impl Into<SharedString>) -> Self {
        Self {
            tag: 2,
            payload: Arc::into_raw(Arc::new(text.into())) as u64,
        }
    }

    /// Borrows the string under the generated-code handle contract.
    ///
    /// # Safety
    /// A string payload must be a live handle created by this runtime.
    pub unsafe fn text(&self) -> &str {
        unsafe { &*(self.payload as *const SharedString) }.as_str()
    }

    /// Renders a valid value using Ocelot display or eval conventions.
    ///
    /// # Safety
    /// String payloads must be live runtime handles.
    pub unsafe fn render(&self, quoted: bool) -> String {
        match self.tag {
            0 => "()".to_owned(),
            1 => (self.payload != 0).to_string(),
            2 if quoted => format!("\"{}\"", unsafe { self.text() }),
            2 => unsafe { self.text() }.to_owned(),
            _ => panic!("invalid native value tag"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NativeValue;
    use crate::runtime_api::{oc_equal, oc_keep, oc_release};

    #[test]
    fn retained_strings_survive_original_owner_release_and_preserve_nul() {
        let original = NativeValue::string("a\0é");
        let copy = unsafe { oc_keep(original) };
        unsafe {
            oc_release(original);
        }
        assert_eq!(unsafe { copy.text() }, "a\0é");
        assert_eq!(unsafe { copy.render(true) }, "\"a\0é\"");
        let same = NativeValue::string("a\0é");
        assert_eq!(unsafe { oc_equal(copy, same) }, 1);
        assert_eq!(
            unsafe { oc_equal(copy, NativeValue { tag: 0, payload: 0 }) },
            0
        );
        unsafe {
            oc_release(copy);
            oc_release(same);
        }
    }
}
