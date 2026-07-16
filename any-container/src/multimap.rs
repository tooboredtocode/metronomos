use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasherDefault;

use crate::boxed::AnyCloneBox;
use crate::utils::IdHasher;
use crate::vec::{AnyVec, AnyVecMutRef};

/// A type-erased multimap storing multiple values per type.
///
/// `AnyMultiMap` allows storing multiple values of the same type, with each
/// type identified by its `TypeId`. Values are stored in vectors behind
/// trait objects, enabling type-erased access while maintaining type safety.
///
/// # Examples
///
/// ```
/// use any_container::AnyMultiMap;
///
/// let mut multimap = AnyMultiMap::new();
/// multimap.insert(1i32);
/// multimap.insert(2i32);
/// multimap.insert("hello".to_string());
///
/// assert_eq!(multimap.len::<i32>(), 2);
/// assert_eq!(multimap.get::<i32>(), &[1, 2]);
///
/// assert_eq!(multimap.len::<String>(), 1);
/// assert_eq!(multimap.get::<String>(), &["hello".to_string()]);
/// ```
#[repr(transparent)]
pub struct AnyMultiMap {
    // A map from a TypeId to a vector of values of that type
    map: HashMap<TypeId, AnyVec, BuildHasherDefault<IdHasher>>,
}

impl Default for AnyMultiMap {
    fn default() -> Self {
        Self::new()
    }
}

impl AnyMultiMap {
    /// Creates a new empty `AnyMultiMap`.
    pub fn new() -> AnyMultiMap {
        AnyMultiMap {
            map: HashMap::default(),
        }
    }

    /// Returns the number of different types stored in the `AnyMultiMap`.
    ///
    /// This counts unique types, not the total number of values.
    pub fn type_count(&self) -> usize {
        self.map.len()
    }

    /// Returns the number of values of type `T` stored in the `AnyMultiMap`.
    pub fn len<T: Any + Send + Sync>(&self) -> usize {
        self.map
            .get(&TypeId::of::<T>())
            .map(|vec| vec.len())
            .unwrap_or(0)
    }

    /// Returns the total number of values stored in the `AnyMultiMap`, across all types.
    pub fn len_total(&self) -> usize {
        self.map.values().map(|vec| vec.len()).sum()
    }

    /// Returns `true` if there are values of type `T` stored in the `AnyMultiMap`.
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.map
            .get(&TypeId::of::<T>())
            .map(|vec| !vec.is_empty())
            .unwrap_or(false)
    }

    /// Returns `true` if there are any values stored in the `AnyMultiMap`, across all types.
    pub fn contains_any(&self) -> bool {
        self.map.values().any(|vec| !vec.is_empty())
    }

    /// Returns `true` if there are no values of type `T` stored in the `AnyMultiMap`.
    pub fn is_empty<T: Any + Send + Sync>(&self) -> bool {
        !self.contains::<T>()
    }

    /// Returns `true` if there are no values stored in the `AnyMultiMap`, across all types.
    pub fn is_completely_empty(&self) -> bool {
        !self.contains_any()
    }

    /// Removes all values of type `T` from the `AnyMultiMap`.
    pub fn clear<T: Any + Send + Sync>(&mut self) {
        self.map.remove(&TypeId::of::<T>());
    }

    /// Removes all values from the `AnyMultiMap`, across all types.
    pub fn clear_all(&mut self) {
        self.map.clear();
    }

    /// Returns a slice of all values of type `T` stored in the `AnyMultiMap`.
    /// If no values of type `T` exist, an empty slice is returned.
    pub fn get<T: Any + Send + Sync>(&self) -> &[T] {
        self.map
            .get(&TypeId::of::<T>())
            .map(|vec| {
                debug_assert_eq!(
                    vec.elem_type_id(),
                    TypeId::of::<T>(),
                    "TypeId mismatch in AnyMultiMap::get. This should never happen!"
                );
                unsafe {
                    // Safety: The invariants guarantee that the vec is of the appropriate type
                    vec.get_unchecked()
                }
            })
            .unwrap_or(&[])
    }

    /// Returns a mutable reference to the vector values of type `T` stored in the `AnyMultiMap`.
    /// If no values of type `T` exist, a new vector is created and returned.
    /// This method is useful for adding multiple values of the same type efficiently.
    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> AnyVecMutRef<'_, T> {
        let vec = self
            .map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| AnyVec::new::<T>());

        debug_assert_eq!(
            vec.elem_type_id(),
            TypeId::of::<T>(),
            "TypeId mismatch in AnyMultiMap::get_mut. This should never happen!"
        );
        unsafe {
            // Safety: The invariants guarantee that the vec is of the appropriate type
            vec.get_mut_unchecked()
        }
    }

    /// Inserts a value of type `T` into the `AnyMultiMap`.
    ///
    /// ## Note
    ///
    /// This is a convenience method that calls `get_mut` and pushes the value into the vector.
    /// If you need to insert multiple values of the same type, consider using `get_mut` directly
    /// to avoid repeated lookups.
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        let mut vec = self.get_mut::<T>();
        vec.push(value);
    }

    /// Inserts a boxed value of any type into the `AnyMultiMap`.
    pub fn insert_boxed(&mut self, value: AnyCloneBox) {
        // Defer do the vtable dispatch to the boxed value, which will handle inserting itself into
        // the multimap.
        value.insert_into_multimap(self);
    }
}

struct TypeCount(usize);

impl fmt::Debug for TypeCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{} entries", self.0))
    }
}

impl fmt::Debug for AnyMultiMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.map
                    .values()
                    .map(|vec| (vec.type_name(), TypeCount(vec.len()))),
            )
            .finish()
    }
}
