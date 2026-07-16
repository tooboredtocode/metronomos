//! [`AnyVec`] implementation and supplemental types.
//!
//! See the [`AnyVec`] documentation for more information.
//!

use std::any::{TypeId, type_name};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::{fmt, mem, ptr, slice};

struct RawVec {
    ptr: *mut u8,
    length: usize,
    cap: usize,
}

impl RawVec {
    fn new_dangling() -> Self {
        Self {
            ptr: ptr::dangling_mut(),
            length: 0,
            cap: 0,
        }
    }

    /// Creates a new `RawVec` from a `Vec<T>`, consuming the vector and taking ownership of its memory.
    fn from_vec<T>(vec: Vec<T>) -> Self {
        let (ptr, length, cap) = vec.into_raw_parts();
        Self {
            ptr: ptr.cast(),
            length,
            cap,
        }
    }

    /// Updates the `RawVec` to point to the memory of a `ManuallyDrop<Vec<T>>`.
    ///
    /// ### Safety
    /// - [`T`] must be the same type that was used to create the `RawVec`.
    /// - The caller must ensure that passed `vec` isn't dropped while the `RawVec` is still in use.
    unsafe fn update_from<T>(&mut self, vec: &mut ManuallyDrop<Vec<T>>) {
        self.ptr = vec.as_mut_ptr().cast();
        self.length = vec.len();
        self.cap = vec.capacity();
    }

    /// Returns a slice of the elements in the `RawVec`.
    ///
    /// ## Safety
    /// [`T`] must be the same type that was used to create the `RawVec`. If the type is different,
    /// this can lead to undefined behaviour.
    unsafe fn as_slice<T>(&self) -> &[T] {
        unsafe {
            // Safety: The caller has ensured that the type `T` matches the type used to create the
            // `RawVec`, and that the memory is valid for reads.
            slice::from_raw_parts(self.ptr.cast(), self.length)
        }
    }

    /// Converts the `RawVec` back into a `Vec<T>`, taking ownership of the memory and ensuring
    /// proper deallocation.
    ///
    /// ## Safety
    /// [`T`] must be the same type that was used to create the `RawVec`. If the type is different,
    /// this can lead to undefined behaviour.
    unsafe fn into_vec<T>(self) -> Vec<T> {
        unsafe { Vec::from_raw_parts(self.ptr.cast(), self.length, self.cap) }
    }

    /// Converts the `RawVec` into a `ManuallyDrop<Vec<T>>`, allowing manage the vector's memory
    /// without automatically dropping it.
    ///
    /// ## Safety
    /// - [`T`] must be the same type that was used to create the `RawVec`.
    /// - The caller must ensure that the returned value does not outlive the memory.
    /// - The caller must ensure that multiple references to the same memory do not occur, as this
    ///   can lead to undefined behaviour.
    unsafe fn as_manually_drop_vec<T>(&self) -> ManuallyDrop<Vec<T>> {
        unsafe { ManuallyDrop::new(Vec::from_raw_parts(self.ptr.cast(), self.length, self.cap)) }
    }
}

/// Drops the `RawVec`, deallocating the memory it owns.
///
/// ## Safety
/// The caller must ensure that the `RawVec` was created from a `Vec<T>`. If the type is different,
/// this can lead to undefined behaviour.
unsafe fn drop_raw_vec<T>(raw_vec: RawVec) {
    let vec = unsafe { raw_vec.into_vec::<T>() };
    drop(vec);
}

/// A type-erased vector that stores values of a single type.
///
/// `AnyVec` enables storing and retrieving vectors of any type that implements
/// `Send + Sync` without knowing the type at compile time. It uses a function pointer
/// for type-specific drop semantics and `TypeId` for runtime type verification.
///
/// # Type Erasure Pattern
///
/// Values are stored behind a trait object and retrieved via type-safe downcasting.
/// The `get::<T>()` method verifies the type at runtime before returning a slice reference.
///
/// # Smart Pointer Types
///
/// The module provides two smart pointer types for vector access:
/// - [`AnyVecRef`] - Immutable reference to a `Vec<T>`
/// - [`AnyVecMutRef`] - Mutable reference to a `Vec<T>`
///
/// # Examples
///
/// ```
/// use any_container::AnyVec;
///
/// let mut vec = AnyVec::new::<i32>();
/// assert!(vec.is_empty());
///
/// vec.get_mut::<i32>().unwrap().push(42);
/// assert_eq!(vec.len(), 1);
///
/// assert_eq!(vec.get::<i32>().unwrap(), &[42]);
/// assert!(vec.get::<f64>().is_none()); // Type mismatch returns None
/// ```
pub struct AnyVec {
    raw_vec: RawVec,
    type_id: TypeId,
    type_name: &'static str,
    drop: unsafe fn(RawVec),
}

// Safety: AnyVec can only be created by Send + Sync types.
unsafe impl Send for AnyVec {}
unsafe impl Sync for AnyVec {}

impl AnyVec {
    /// Creates a new empty `AnyVec` for elements of type `T`.
    pub fn new<T: 'static + Send + Sync>() -> Self {
        Self::from_vec(Vec::<T>::new())
    }

    /// Creates a new `AnyVec` with the specified capacity for elements of type `T`.
    pub fn new_with_capacity<T: 'static + Send + Sync>(capacity: usize) -> Self {
        Self::from_vec(Vec::<T>::with_capacity(capacity))
    }

    /// Creates a new `AnyVec` from a `Vec<T>`, consuming the vector and taking ownership of its memory.
    pub fn from_vec<T: 'static + Send + Sync>(vec: Vec<T>) -> Self {
        Self {
            raw_vec: RawVec::from_vec(vec),
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
            drop: drop_raw_vec::<T>,
        }
    }

    /// Returns the type id of the elements stored in the `AnyVec`.
    pub fn elem_type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the type name of the elements stored in the `AnyVec`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns a reference to the elements in the `AnyVec` as a slice of type `T`.
    /// Returns `None` if the stored type doesn't match `T`.
    pub fn get<T: 'static>(&self) -> Option<&[T]> {
        if self.type_id == TypeId::of::<T>() {
            unsafe {
                // Safety: We just checked that the type `T` matches the type of the elements
                // stored in the `AnyVec`, so it is safe to call `get_unchecked`.
                Some(self.get_unchecked::<T>())
            }
        } else {
            None
        }
    }

    /// Returns a reference to the elements in the `AnyVec` as a slice of type `T`.
    ///
    /// ## Safety
    /// The caller must ensure that the type `T` matches the type of the elements stored in the
    /// `AnyVec`. If the type is different, this can lead to undefined behaviour.
    pub unsafe fn get_unchecked<T: 'static>(&self) -> &[T] {
        unsafe { self.raw_vec.as_slice::<T>() }
    }

    /// Returns a smart pointer, dereferencing to a `Vec<T>`, allowing access to the elements in
    /// the `AnyVec` as a vector of type `T`.
    ///
    /// ## Note
    /// In most cases, you want [`AnyVec::get`] or [`AnyVec::get_mut`] instead.
    pub fn get_ref<T: 'static>(&self) -> Option<AnyVecRef<'_, T>> {
        if self.type_id == TypeId::of::<T>() {
            unsafe {
                // Safety: We just checked that the type `T` matches the type of the elements
                // stored in the `AnyVec`, so it is safe to call `get_unchecked`.
                Some(AnyVecRef::new(self))
            }
        } else {
            None
        }
    }

    /// Returns a smart pointer, dereferencing to a `Vec<T>`, allowing access to the elements in
    /// the `AnyVec` as a vector of type `T`.
    ///
    /// ## Note
    /// In most cases, you want [`AnyVec::get_unchecked`] or [`AnyVec::get_mut_unchecked`] instead.
    ///
    /// ## Safety
    /// The caller must ensure that the type `T` matches the type of the elements stored in the
    /// `AnyVec`. If the type is different, this can lead to undefined behaviour.
    pub unsafe fn get_ref_unchecked<T: 'static>(&self) -> AnyVecRef<'_, T> {
        unsafe { AnyVecRef::new(self) }
    }

    /// Returns a smart pointer, dereferencing to a mutable `Vec<T>`, allowing access to the elements in
    /// the `AnyVec` as a mutable vector of type `T`.
    pub fn get_mut<T: 'static>(&mut self) -> Option<AnyVecMutRef<'_, T>> {
        if self.type_id == TypeId::of::<T>() {
            unsafe {
                // Safety: We just checked that the type `T` matches the type of the elements
                // stored in the `AnyVec`, so it is safe to call `get_unchecked`.
                Some(AnyVecMutRef::new(self))
            }
        } else {
            None
        }
    }

    /// Returns a smart pointer, dereferencing to a mutable `Vec<T>`, allowing access to the elements in
    /// the `AnyVec` as a mutable vector of type `T`.
    ///
    /// ## Safety
    /// The caller must ensure that the type `T` matches the type of the elements stored in the
    /// `AnyVec`. If the type is different, this can lead to undefined behaviour.
    pub unsafe fn get_mut_unchecked<T: 'static>(&mut self) -> AnyVecMutRef<'_, T> {
        unsafe { AnyVecMutRef::new(self) }
    }

    /// Returns the number of elements in the `AnyVec`.
    pub fn len(&self) -> usize {
        self.raw_vec.length
    }

    /// Returns true if the `AnyVec` contains no elements.
    pub fn is_empty(&self) -> bool {
        self.raw_vec.length == 0
    }

    /// Consumes the `AnyVec` and returns the elements as a `Vec<T>`.
    pub fn try_into_vec<T: 'static>(self) -> Result<Vec<T>, Self> {
        if self.type_id == TypeId::of::<T>() {
            Ok(unsafe { self.into_vec_unchecked::<T>() })
        } else {
            Err(self)
        }
    }

    /// Consumes the `AnyVec` and returns the elements as a `Vec<T>`.
    ///
    /// ## Safety
    /// The caller must ensure that the type `T` matches the type of the elements stored in the
    /// `AnyVec`. If the type is different, this can lead to undefined behaviour.
    pub unsafe fn into_vec_unchecked<T: 'static>(mut self) -> Vec<T> {
        let raw_vec = mem::replace(&mut self.raw_vec, RawVec::new_dangling());
        mem::forget(self); // No need to drop self, as we are taking ownership of the raw_vec
        unsafe {
            // Safety: The caller has ensured that the type `T` matches the type of the elements
            // stored in the `AnyVec`, so it is safe to call `into_vec`.
            raw_vec.into_vec::<T>()
        }
    }
}

impl fmt::Debug for AnyVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyVec")
            .field("type", &self.type_name)
            .field("length", &self.raw_vec.length)
            .finish()
    }
}

impl Drop for AnyVec {
    fn drop(&mut self) {
        let inner = mem::replace(&mut self.raw_vec, RawVec::new_dangling());
        unsafe {
            // Safety: The drop function was created with the correct type when the AnyVec was
            // created, so it is safe to call it here.
            (self.drop)(inner);
        }
    }
}

/// A smart pointer that dereferences to a `Vec<T>`.
///
/// `AnyVecRef` is created from an `AnyVec` and provides vector-like access to
/// the underlying values. It is returned by methods like `AnyVec::get_ref`.
pub struct AnyVecRef<'a, T> {
    raw: &'a AnyVec,
    vec: ManuallyDrop<Vec<T>>,
}

/// A smart pointer that dereferences to a mutable `Vec<T>`.
///
/// `AnyVecMutRef` is created from a mutable `AnyVec` and provides vector-like access
/// to the underlying values. It is returned by methods like `AnyVec::get_mut`.
/// Upon drop, it updates the original `AnyVec` with any modifications.
pub struct AnyVecMutRef<'a, T> {
    raw: &'a mut AnyVec,
    vec: ManuallyDrop<Vec<T>>,
}

unsafe impl<'a, T: Send> Send for AnyVecRef<'a, T> {}
unsafe impl<'a, T: Sync> Sync for AnyVecRef<'a, T> {}
unsafe impl<'a, T: Send> Send for AnyVecMutRef<'a, T> {}
unsafe impl<'a, T: Sync> Sync for AnyVecMutRef<'a, T> {}

impl<'a, T> AnyVecRef<'a, T> {
    /// Creates a new `AnyVecRef` from an `AnyVec`.
    ///
    /// ## Safety
    /// The caller must ensure that the `AnyVec` contains values of type `T`.
    unsafe fn new(raw: &'a AnyVec) -> Self {
        let vec = unsafe { raw.raw_vec.as_manually_drop_vec::<T>() };
        Self { raw, vec }
    }
}

impl<'a, T> AnyVecMutRef<'a, T> {
    /// Creates a new `AnyVecMutRef` from a mutable `AnyVec`.
    ///
    /// ## Safety
    /// The caller must ensure that the `AnyVec` contains values of type `T`.
    unsafe fn new(raw: &'a mut AnyVec) -> Self {
        let vec = unsafe { raw.raw_vec.as_manually_drop_vec::<T>() };
        Self { raw, vec }
    }
}

impl<'a, T> Clone for AnyVecRef<'a, T> {
    fn clone(&self) -> Self {
        unsafe {
            // Safety: AnyVecRef is only created from a reference to AnyVec, and the caller
            // has already ensured upon creation that the types match.
            Self::new(self.raw)
        }
    }
}

impl<'a, T> Deref for AnyVecRef<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<'a, T> Deref for AnyVecMutRef<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<'a, T> DerefMut for AnyVecMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for AnyVecRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.vec.fmt(f)
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for AnyVecMutRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.vec.fmt(f)
    }
}

impl<'a, T> Drop for AnyVecMutRef<'a, T> {
    fn drop(&mut self) {
        unsafe {
            // Safety: AnyVecMutRef is only created from a mutable reference to AnyVec, and the caller
            // has already ensured upon creation that the types match.
            self.raw.raw_vec.update_from(&mut self.vec);
        }
    }
}
