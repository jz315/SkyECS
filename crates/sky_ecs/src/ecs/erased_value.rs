use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::mem::{self, MaybeUninit};
use std::ptr::{self, NonNull};

const INLINE_VALUE_BYTES: usize = 16;

#[repr(align(16))]
pub(crate) struct InlineValueBytes([MaybeUninit<u8>; INLINE_VALUE_BYTES]);

/// Owned type-erased component bytes shared by deferred commands and the
/// dynamic ECS API. Small values stay inline; larger or over-aligned values
/// use an exact heap allocation.
pub(crate) enum InsertValue {
    Inline {
        len: usize,
        bytes: InlineValueBytes,
        drop_fn: Option<unsafe fn(*mut u8)>,
    },
    Heap {
        data: NonNull<u8>,
        len: usize,
        layout: Layout,
        drop_fn: Option<unsafe fn(*mut u8)>,
    },
    Consumed,
}

impl InsertValue {
    pub(crate) fn from_value<T: 'static>(value: T) -> Self {
        let len = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        let drop_fn = mem::needs_drop::<T>()
            .then_some(sky_type::drop_in_place_erased::<T> as unsafe fn(*mut u8));
        let value = mem::ManuallyDrop::new(value);

        if len <= INLINE_VALUE_BYTES && align <= mem::align_of::<InlineValueBytes>() {
            let mut bytes = InlineValueBytes([MaybeUninit::uninit(); INLINE_VALUE_BYTES]);
            unsafe {
                ptr::copy_nonoverlapping(
                    (&*value as *const T).cast::<u8>(),
                    bytes.0.as_mut_ptr().cast::<u8>(),
                    len,
                );
            }
            Self::Inline {
                len,
                bytes,
                drop_fn,
            }
        } else {
            let layout = Layout::from_size_align(len.max(1), align).unwrap();
            let data = unsafe {
                let raw = alloc(layout);
                NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout))
            };
            unsafe {
                ptr::copy_nonoverlapping((&*value as *const T).cast::<u8>(), data.as_ptr(), len);
            }
            Self::Heap {
                data,
                len,
                layout,
                drop_fn,
            }
        }
    }

    #[inline(always)]
    pub(crate) fn write(&mut self, dst: *mut u8) {
        unsafe {
            match self {
                Self::Inline {
                    len,
                    bytes,
                    drop_fn,
                } => {
                    ptr::copy_nonoverlapping(bytes.0.as_ptr().cast::<u8>(), dst, *len);
                    *drop_fn = None;
                }
                Self::Heap {
                    data, len, drop_fn, ..
                } => {
                    ptr::copy_nonoverlapping(data.as_ptr(), dst, *len);
                    *drop_fn = None;
                }
                Self::Consumed => {
                    debug_assert!(false, "InsertValue::write called on a consumed value");
                    return;
                }
            }
        }
        *self = Self::Consumed;
    }
}

impl Drop for InsertValue {
    fn drop(&mut self) {
        match self {
            Self::Inline {
                bytes,
                drop_fn: Some(drop_fn),
                ..
            } => unsafe {
                drop_fn(bytes.0.as_mut_ptr().cast::<u8>());
            },
            Self::Heap {
                data,
                layout,
                drop_fn,
                ..
            } => {
                // The allocation is library-owned and must be released even
                // when a user component destructor panics.
                let result = drop_fn.take().map(|drop_fn| {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                        drop_fn(data.as_ptr());
                    }))
                });
                unsafe {
                    dealloc(data.as_ptr(), *layout);
                }
                if let Some(Err(payload)) = result {
                    std::panic::resume_unwind(payload);
                }
            }
            _ => {}
        }
    }
}
