use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::hash::BuildHasherDefault;

use crate::boxed::AnyCloneBox;
use crate::utils::IdHasher;

/// A type-erased map storing values of different types by their `TypeId`.
///
/// `AnyMap` allows storing and retrieving values of any type that implements
/// `Clone + Send + Sync`. The map uses `TypeId` as keys, enabling type-safe
/// lookups while maintaining type erasure through boxed trait objects.
///
/// # Examples
///
/// ```
/// use any_container::AnyMap;
///
/// let mut map = AnyMap::new();
/// map.insert(42i32);
/// map.insert("world".to_string());
///
/// assert_eq!(map.len(), 2);
/// assert_eq!(*map.get::<i32>().unwrap(), 42);
/// ```
#[repr(transparent)]
#[derive(Clone)]
pub struct AnyMap {
    // A map from a TypeId to a Box of a type
    map: HashMap<TypeId, AnyCloneBox, BuildHasherDefault<IdHasher>>,
}

impl Default for AnyMap {
    fn default() -> Self {
        Self::new()
    }
}

impl AnyMap {
    /// Creates a new empty `AnyMap`.
    pub fn new() -> AnyMap {
        AnyMap {
            map: HashMap::default(),
        }
    }

    /// Creates a new empty `AnyMap` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> AnyMap {
        AnyMap {
            map: HashMap::with_capacity_and_hasher(
                capacity,
                BuildHasherDefault::<IdHasher>::default(),
            ),
        }
    }

    /// Returns the number of elements that the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Reserves capacity for at least `additional` more elements in the map.
    /// The map may reserve more space than requested to avoid frequent reallocations.
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    /// Shrinks the capacity of the map as much as possible.
    /// This reduces the allocated memory to fit the current contents.
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Removes all elements from the map.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns a reference to the value corresponding to the type `T`.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).map(|elem| unsafe {
            debug_assert_eq!(
                AnyCloneBox::type_id(elem),
                TypeId::of::<T>(),
                "TypeId mismatch in AnyMap::get. This should never happen!"
            );
            // Safety: The invariants guarantee that the type of the value is the same as the type of the key.
            elem.downcast_ref_unchecked()
        })
    }

    /// Returns a mutable reference to the value corresponding to the type `T`.
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>()).map(|elem| unsafe {
            debug_assert_eq!(
                AnyCloneBox::type_id(elem),
                TypeId::of::<T>(),
                "TypeId mismatch in AnyMap::get_mut. This should never happen!"
            );
            // Safety: The invariants guarantee that the type of the value is the same as the type of the key.
            elem.downcast_mut_unchecked()
        })
    }

    // Inserts a value of type `T` into the map. If a value of type `T` already exists, it will be
    // replaced and the old value will be returned.
    pub fn insert<T: Any + Clone + Send + Sync>(&mut self, value: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), AnyCloneBox::new(value))
            .map(|old_value| unsafe {
                debug_assert_eq!(
                    AnyCloneBox::type_id(&old_value),
                    TypeId::of::<T>(),
                    "TypeId mismatch in AnyMap::insert. This should never happen!"
                );
                // Safety: The invariants guarantee that the type of the value is the same as the type of the key.
                *old_value.downcast_unchecked()
            })
    }

    /// Inserts a boxed value into the map.
    /// If a value of the same `TypeId` already exists, it will be replaced and the old value
    /// will be returned.
    /// This method is useful when you already have an `AnyCloneBox` instance.
    pub fn insert_boxed(&mut self, value: AnyCloneBox) -> Option<AnyCloneBox> {
        self.map.insert(value.type_id(), value)
    }

    /// Inserts a value of type `T` into the map. If a value of type `T` already exists,
    /// it will not be replaced and the new value will be returned as an error.
    pub fn try_insert<T: Any + Clone + Send + Sync>(&mut self, value: T) -> Result<(), T> {
        match self.map.entry(TypeId::of::<T>()) {
            Entry::Occupied(_) => Err(value),
            Entry::Vacant(entry) => {
                entry.insert(AnyCloneBox::new(value));
                Ok(())
            }
        }
    }

    /// Inserts a boxed value into the map. If a value of the same type already exists,
    /// it will not be replaced and the new value will be returned as an error.
    pub fn try_insert_boxed(&mut self, value: AnyCloneBox) -> Result<(), AnyCloneBox> {
        match self.map.entry(value.type_id()) {
            Entry::Occupied(_) => Err(value),
            Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
        }
    }

    /// Removes the stored value of type `T` from the map and returns it, if it exists.
    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.map.remove(&TypeId::of::<T>()).map(|old_value| unsafe {
            debug_assert_eq!(
                AnyCloneBox::type_id(&old_value),
                TypeId::of::<T>(),
                "TypeId mismatch in AnyMap::remove. This should never happen!"
            );
            // Safety: The invariants guarantee that the type of the value is the same as the type of the key.
            *old_value.downcast_unchecked()
        })
    }

    /// Returns true if the map contains a value of type `T`.
    pub fn contains<T: Any>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
}

impl fmt::Debug for AnyMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.map.values().map(|any_clone| any_clone.type_name()))
            .finish()
    }
}
