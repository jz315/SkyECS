use std::any::Any;
use std::mem;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

pub(crate) type PanicPayload = Box<dyn Any + Send + 'static>;

/// Drops a panic payload without allowing an adversarial payload destructor
/// to start a second unwind.
pub(crate) fn dispose_panic_payload_without_unwinding(payload: PanicPayload) {
    if let Err(nested_payload) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
        // Propagating a panic raised while disposing another panic payload can
        // abort the process. This last-resort leak is limited to the malicious
        // nested payload itself.
        mem::forget(nested_payload);
    }
}

/// Runs cleanup code and prevents its panic from escaping.
pub(crate) fn suppress_unwind(f: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(f)) {
        dispose_panic_payload_without_unwinding(payload);
    }
}

/// Runs every cleanup task, preserving the first panic and suppressing later
/// cleanup failures until all tasks have been attempted.
#[derive(Default)]
pub(crate) struct PanicAccumulator {
    first: Option<PanicPayload>,
}

impl PanicAccumulator {
    pub(crate) fn run(&mut self, f: impl FnOnce()) {
        if let Err(payload) = catch_unwind(AssertUnwindSafe(f)) {
            if self.first.is_none() {
                self.first = Some(payload);
            } else {
                dispose_panic_payload_without_unwinding(payload);
            }
        }
    }

    pub(crate) fn resume_if_any(self) {
        if let Some(payload) = self.first {
            resume_unwind(payload);
        }
    }
}
