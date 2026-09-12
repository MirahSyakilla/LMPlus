//! IL2CPP List definition and operations
use super::array::Il2cppArray;
use std::ffi::c_void;
use std::marker::PhantomData;

#[repr(C)]
pub struct Il2cppList<T: Copy> {
    /// Pointer to the list class
    pub klass: *mut c_void,
    /// Monitor for synchronization
    pub monitor: *mut c_void,
    /// Internal array of items
    pub items: *mut Il2cppArray<T>,
    /// Number of elements in the list
    pub size: i32,
    /// Version of the list
    pub version: i32,
    _phantom: PhantomData<T>,
}

impl<T: Copy> Il2cppList<T> {
    /// Returns the logical list length, clamping invalid negative metadata to zero.
    #[inline]
    pub fn len(&self) -> usize {
        self.size.max(0) as usize
    }

    /// Returns true if this list has no logical elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn bounded_len(&self) -> usize {
        self.items_array()
            .map(|arr| self.len().min(arr.max_length))
            .unwrap_or(0)
    }

    /// Gets the internal items array
    ///
    /// # Returns
    /// * `Option<&Il2cppArray<T>>` - The underlying array, or None if null
    pub fn items_array(&self) -> Option<&Il2cppArray<T>> {
        if self.items.is_null() {
            None
        } else {
            unsafe { Some(&*self.items) }
        }
    }

    /// Gets the internal items array as mutable
    ///
    /// # Returns
    /// * `Option<&mut Il2cppArray<T>>` - The underlying mutable array, or None if null
    pub fn items_array_mut(&mut self) -> Option<&mut Il2cppArray<T>> {
        if self.items.is_null() {
            None
        } else {
            unsafe { Some(&mut *self.items) }
        }
    }

    /// Gets the element at the specified index
    ///
    /// # Arguments
    /// * `index` - The index of the element to retrieve
    ///
    /// # Returns
    /// * `Option<T>` - The element if present and index is within bounds, otherwise None
    pub fn get(&self, index: usize) -> Option<T> {
        if index >= self.len() {
            return None;
        }
        self.items_array()
            .and_then(|arr| (index < arr.max_length).then(|| arr.get(index)))
    }

    /// Alias for `get` that panics on out of bounds
    ///
    /// # Arguments
    /// * `index` - The index of the element to retrieve
    ///
    /// # Returns
    /// * `T` - The element at the specified index
    ///
    /// # Panics
    /// Panics if the index is out of bounds or the item array is null.
    pub fn at(&self, index: usize) -> T {
        self.get(index).expect("Index out of bounds")
    }

    /// Sets the element at the specified index
    ///
    /// # Arguments
    /// * `index` - The index where the value should be set
    /// * `value` - The value to set
    ///
    /// # Returns
    /// * `bool` - True if setting was successful, False if index out of bounds or array null
    pub fn set(&mut self, index: usize, value: T) -> bool {
        if index >= self.len() {
            return false;
        }
        if let Some(arr) = self.items_array_mut() {
            if index >= arr.max_length {
                return false;
            }
            arr.set(index, value);
            true
        } else {
            false
        }
    }

    /// Converts the list to a Rust Vec
    ///
    /// # Returns
    /// * `Vec<T>` - A new vector containing the list elements
    pub fn to_vec(&self) -> Vec<T> {
        let Some(arr) = self.items_array() else {
            return Vec::new();
        };
        let len = self.len().min(arr.max_length);
        unsafe { std::slice::from_raw_parts(arr.get_pointer(), len).to_vec() }
    }

    /// Gets a pointer to the data array
    ///
    /// # Returns
    /// * `Option<*const T>` - Pointer to the first element, or None
    pub fn get_pointer(&self) -> Option<*const T> {
        self.items_array().map(|arr| arr.get_pointer())
    }

    /// Returns an iterator over the list items
    ///
    /// # Returns
    /// * `impl Iterator<Item = T> + '_` - An iterator yielding elements
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.bounded_len()).filter_map(|i| self.get(i))
    }
}
