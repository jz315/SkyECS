#![deny(unsafe_op_in_unsafe_fn)]

use rustc_hash::FxHashMap;
use std::any::TypeId;
use std::ptr::NonNull;

struct ResourceSlot {
    ptr: NonNull<u8>,
    drop_fn: unsafe fn(NonNull<u8>),
}

impl ResourceSlot {
    fn new<R: 'static>(resource: R) -> Self {
        let ptr = NonNull::new(Box::into_raw(Box::new(resource)))
            .expect("Box::into_raw must return a non-null resource pointer")
            .cast();
        Self {
            ptr,
            drop_fn: drop_resource::<R>,
        }
    }

    /// Returns the allocation-rooted pointer owned by this slot.
    ///
    /// # Safety
    ///
    /// `R` must be the type used to construct this slot. The Resources map
    /// maintains that invariant by keying every slot with `TypeId::of::<R>()`.
    unsafe fn ptr<R: 'static>(&self) -> *mut R {
        self.ptr.cast::<R>().as_ptr()
    }

    /// # Safety
    ///
    /// `R` must be the type used to construct this slot, and no mutable
    /// reference to the stored value may be live.
    unsafe fn get<R: 'static>(&self) -> &R {
        // SAFETY: guaranteed by the method contract. The pointer originates
        // from the slot's owning Box allocation rather than from an `&R`.
        unsafe { self.ptr.cast::<R>().as_ref() }
    }

    /// # Safety
    ///
    /// `R` must be the type used to construct this slot. The exclusive slot
    /// borrow must cover the returned reference.
    unsafe fn get_mut<R: 'static>(&mut self) -> &mut R {
        // SAFETY: guaranteed by the method contract and the exclusive slot
        // borrow.
        unsafe { self.ptr.cast::<R>().as_mut() }
    }

    /// # Safety
    ///
    /// `R` must be the type used to construct this slot.
    unsafe fn into_value<R: 'static>(self) -> R {
        let ptr = self.ptr.cast::<R>().as_ptr();
        std::mem::forget(self);
        // SAFETY: the slot uniquely owns the allocation, and forgetting the
        // slot prevents its Drop implementation from freeing it a second time.
        unsafe { *Box::from_raw(ptr) }
    }
}

impl Drop for ResourceSlot {
    fn drop(&mut self) {
        // SAFETY: `drop_fn` was paired with this allocation in `new`, and a
        // live ResourceSlot uniquely owns that allocation.
        unsafe { (self.drop_fn)(self.ptr) };
    }
}

unsafe fn drop_resource<R>(ptr: NonNull<u8>) {
    // SAFETY: callers guarantee that `ptr` owns a Box allocation containing R.
    unsafe { drop(Box::from_raw(ptr.cast::<R>().as_ptr())) };
}

#[derive(Default)]
pub(crate) struct Resources {
    values: FxHashMap<TypeId, ResourceSlot>,
}

impl Resources {
    pub(crate) fn insert<R: 'static>(&mut self, resource: R) -> Option<R> {
        self.values
            .insert(TypeId::of::<R>(), ResourceSlot::new(resource))
            // SAFETY: every slot is stored under the TypeId used to construct
            // it, so a replaced slot at this key contains R.
            .map(|old| unsafe { old.into_value::<R>() })
    }

    pub(crate) fn get<R: 'static>(&self) -> Option<&R> {
        self.values
            .get(&TypeId::of::<R>())
            // SAFETY: the map key enforces the slot's concrete type, and this
            // shared Resources borrow cannot produce a mutable safe access.
            .map(|slot| unsafe { slot.get::<R>() })
    }

    pub(crate) fn get_mut<R: 'static>(&mut self) -> Option<&mut R> {
        self.values
            .get_mut(&TypeId::of::<R>())
            // SAFETY: the map key enforces the slot's concrete type, and the
            // exclusive Resources borrow covers the returned reference.
            .map(|slot| unsafe { slot.get_mut::<R>() })
    }

    pub(crate) fn contains<R: 'static>(&self) -> bool {
        self.values.contains_key(&TypeId::of::<R>())
    }

    pub(crate) fn contains_id(&self, id: TypeId) -> bool {
        self.values.contains_key(&id)
    }

    pub(crate) fn remove<R: 'static>(&mut self) -> Option<R> {
        self.values
            .remove(&TypeId::of::<R>())
            // SAFETY: a removed slot at this key was constructed with R.
            .map(|slot| unsafe { slot.into_value::<R>() })
    }

    /// Returns a pointer rooted at the erased slot's owning allocation.
    /// Scheduler access validation determines whether callers may dereference
    /// it as shared or exclusive during a system wave.
    pub(crate) fn ptr<R: 'static>(&self) -> Option<*mut R> {
        self.values.get(&TypeId::of::<R>()).map(|slot| {
            // SAFETY: every slot is stored under its concrete TypeId. This
            // returns the allocation pointer without first creating `&R`.
            unsafe { slot.ptr::<R>() }
        })
    }
}
