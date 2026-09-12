//! IL2CPP Dictionary definition and operations
use super::array::Il2cppArray;
use std::ffi::c_void;
use std::marker::PhantomData;

#[repr(C)]
pub struct Il2cppDictionary<K: Copy, V: Copy> {
    /// Pointer to the dictionary class
    pub klass: *mut c_void,
    /// Monitor for synchronization
    pub monitor: *mut c_void,
    /// Array of bucket indices
    pub buckets: *mut Il2cppArray<i32>,
    /// Array of entries
    pub entries: *mut Il2cppArray<c_void>,
    /// Array of keys
    pub keys: *mut Il2cppArray<K>,
    /// Array of values
    pub values: *mut Il2cppArray<V>,
    /// Number of touched slots
    pub touched_slots: i32,
    /// Index of the first empty slot
    pub empty_slot: i32,
    /// Number of elements in the dictionary
    pub count: i32,
    _phantom_k: PhantomData<K>,
    _phantom_v: PhantomData<V>,
}

impl<K: Copy, V: Copy> Il2cppDictionary<K, V> {
    /// Gets the keys array
    ///
    /// # Returns
    /// * `Option<&Il2cppArray<K>>` - The array of keys, or None if null
    pub fn keys_array(&self) -> Option<&Il2cppArray<K>> {
        if self.keys.is_null() {
            None
        } else {
            unsafe { Some(&*self.keys) }
        }
    }

    /// Gets the values array
    ///
    /// # Returns
    /// * `Option<&Il2cppArray<V>>` - The array of values, or None if null
    pub fn values_array(&self) -> Option<&Il2cppArray<V>> {
        if self.values.is_null() {
            None
        } else {
            unsafe { Some(&*self.values) }
        }
    }

    /// Gets keys as a Rust Vec
    ///
    /// # Returns
    /// * `Vec<K>` - A vector containing all keys
    pub fn get_keys(&self) -> Vec<K> {
        self.keys_array()
            .map(|arr| arr.to_vector())
            .unwrap_or_default()
    }

    /// Gets values as a Rust Vec
    ///
    /// # Returns
    /// * `Vec<V>` - A vector containing all values
    pub fn get_values(&self) -> Vec<V> {
        self.values_array()
            .map(|arr| arr.to_vector())
            .unwrap_or_default()
    }

    /// Gets a pointer to the keys array data
    ///
    /// # Returns
    /// * `Option<*const K>` - Pointer to the first key, or None
    pub fn keys_pointer(&self) -> Option<*const K> {
        self.keys_array().map(|arr| arr.get_pointer())
    }

    /// Gets a pointer to the values array data
    ///
    /// # Returns
    /// * `Option<*const V>` - Pointer to the first value, or None
    pub fn values_pointer(&self) -> Option<*const V> {
        self.values_array().map(|arr| arr.get_pointer())
    }
}
