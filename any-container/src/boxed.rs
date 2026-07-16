use std::any::{Any, TypeId, type_name};
use std::ops::{Deref, DerefMut};

use crate::multimap::AnyMultiMap;

/// A type-erased cloneable box that stores any `Clone + Send + Sync` type.
///
/// `AnyCloneBox` enables runtime type-erased storage of values while maintaining
/// the ability to clone and downcast them to their original types. It is the
/// fundamental building block for type-erased containers in the crate.
///
/// # Examples
///
/// ```
/// use any_container::AnyCloneBox;
///
/// // Create an AnyCloneBox with an i32
/// let boxed = AnyCloneBox::new(42i32);
///
/// // Downcast to get the original value
/// assert_eq!(*boxed.downcast_ref::<i32>().unwrap(), 42);
///
/// // Downcasting to a wrong type returns None
/// assert!(boxed.downcast_ref::<String>().is_none());
///
/// // Clone the boxed value
/// let _clone = boxed.clone();
/// ```
pub struct AnyCloneBox {
    inner: Box<dyn AnyClone>,
    type_id: TypeId,
    type_name: &'static str,
}

impl AnyCloneBox {
    /// Creates a new `AnyCloneBox` containing the given value.
    pub fn new<T: Clone + Send + Sync + 'static>(value: T) -> Self {
        AnyCloneBox {
            inner: Box::new(value),
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
        }
    }

    /// Returns the `TypeId` of the value contained in this `AnyCloneBox`.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the type name of the value contained in this `AnyCloneBox`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Attempts to downcast the `AnyCloneBox` to an immutable reference of type `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.type_id == TypeId::of::<T>() {
            Some(unsafe {
                // SAFETY: The type has been checked to match, so this is safe.
                self.downcast_ref_unchecked::<T>()
            })
        } else {
            None
        }
    }

    /// Unsafely downcasts the `AnyCloneBox` to an immutable reference of type `T` without checking the type.
    ///
    /// # Safety
    /// The caller must ensure that the type `T` is the same as the type of the value contained in
    /// this `AnyCloneBox`. If the types do not match, this will result in undefined behaviour.
    pub unsafe fn downcast_ref_unchecked<T: Any>(&self) -> &T {
        unsafe { &*(self.inner.deref() as *const dyn Any as *const T) }
    }

    /// Attempts to downcast the `AnyCloneBox` to a mutable reference of type `T`.
    ///
    /// This method allows mutation of the contained value through a type-safe interface.
    /// Returns `None` if the contained type doesn't match `T`.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.type_id == TypeId::of::<T>() {
            Some(unsafe {
                // SAFETY: The type has been checked to match, so this is safe.
                self.downcast_mut_unchecked::<T>()
            })
        } else {
            None
        }
    }

    /// Unsafely downcasts the `AnyCloneBox` to a mutable reference of type `T` without checking the type.
    ///
    /// # Safety
    /// The caller must ensure that the type `T` is the same as the type of the value contained in
    /// this `AnyCloneBox`. If the types do not match, this will result in undefined behaviour.
    pub unsafe fn downcast_mut_unchecked<T: Any>(&mut self) -> &mut T {
        unsafe { &mut *(self.inner.deref_mut() as *mut dyn Any as *mut T) }
    }

    /// Consumes the `AnyCloneBox` and attempts to downcast it to a boxed value of type `T`.
    pub fn downcast<T: Any>(self) -> Result<Box<T>, Self> {
        if self.type_id == TypeId::of::<T>() {
            Ok(unsafe {
                // SAFETY: The type has been checked to match, so this is safe.
                self.downcast_unchecked::<T>()
            })
        } else {
            Err(self)
        }
    }

    /// Unsafely downcasts the `AnyCloneBox` to a boxed value of type `T` without checking the type.
    ///
    /// # Safety
    /// The caller must ensure that the type `T` is the same as the type of the value contained in
    /// this `AnyCloneBox`. If the types do not match, this will result in undefined behaviour.
    pub unsafe fn downcast_unchecked<T: Any>(self) -> Box<T> {
        unsafe {
            let raw = Box::into_raw(self.inner);
            Box::from_raw(raw as *mut T)
        }
    }
}

impl AnyCloneBox {
    pub(super) fn insert_into_multimap(self, multimap: &mut AnyMultiMap) {
        self.inner.insert_into_multimap(multimap);
    }
}

impl Clone for AnyCloneBox {
    fn clone(&self) -> Self {
        AnyCloneBox {
            inner: self.inner.clone_box(),
            type_id: self.type_id,
            type_name: self.type_name,
        }
    }
}

trait AnyClone: Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn AnyClone>;
    fn insert_into_multimap(self: Box<Self>, multimap: &mut AnyMultiMap);
}

impl<T: Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_box(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
    fn insert_into_multimap(self: Box<Self>, multimap: &mut AnyMultiMap) {
        multimap.insert(*self);
    }
}
