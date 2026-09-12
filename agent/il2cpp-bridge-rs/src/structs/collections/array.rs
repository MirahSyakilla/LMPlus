//! IL2CPP Array definition and operations
use crate::api;
use crate::structs::core::Class;
use std::ffi::c_void;
use std::marker::PhantomData;

#[repr(C)]
pub struct ArrayBounds {
    /// Length of the dimension
    pub length: usize,
    /// Lower bound of the dimension
    pub lower_bound: i32,
}

#[repr(C)]
pub struct Il2cppArray<T> {
    /// Pointer to the array class
    pub klass: *mut c_void,
    /// Monitor for synchronization
    pub monitor: *mut c_void,
    /// Pointer to array bounds
    pub bounds: *mut ArrayBounds,
    /// Maximum length of the array
    pub max_length: usize,
    phantom: PhantomData<T>,
}

impl<T: Copy> Il2cppArray<T> {
    /// Returns the number of elements in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.max_length
    }

    /// Returns true if this array has no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.max_length == 0
    }

    /// Gets the address of the data array
    ///
    /// # Returns
    /// * `usize` - The memory address where the array data begins
    #[inline]
    pub fn get_data(&self) -> usize {
        let header_size = std::mem::size_of::<Self>();
        (self as *const Self as usize) + header_size
    }

    /// Gets the element at the specified index
    ///
    /// # Arguments
    /// * `index` - The index of the element to retrieve
    ///
    /// # Returns
    /// * `T` - The element at the specified index
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn get(&self, index: usize) -> T {
        if index >= self.max_length {
            panic!("Index out of bounds: {} >= {}", index, self.max_length);
        }

        unsafe { self.get_pointer().add(index).read() }
    }

    /// Alias for `get`
    ///
    /// # Arguments
    /// * `index` - The index of the element to retrieve
    ///
    /// # Returns
    /// * `T` - The element at the specified index
    pub fn at(&self, index: usize) -> T {
        self.get(index)
    }

    /// Sets the element at the specified index
    ///
    /// # Arguments
    /// * `index` - The index where the value should be set
    /// * `value` - The value to set
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn set(&mut self, index: usize, value: T) {
        if index >= self.max_length {
            panic!("Index out of bounds: {} >= {}", index, self.max_length);
        }

        unsafe { (self.get_pointer() as *mut T).add(index).write(value) }
    }

    /// Gets a raw pointer to the data
    ///
    /// # Returns
    /// * `*const T` - Pointer to the first element of the array
    pub fn get_pointer(&self) -> *const T {
        let header_size = std::mem::size_of::<Self>();
        ((self as *const Self as usize) + header_size) as *const T
    }

    /// Inserts elements from a slice into the array
    ///
    /// # Arguments
    /// * `arr` - The slice of elements to insert
    /// * `size` - The number of elements to insert
    /// * `index` - The starting index in the array
    pub fn insert(&mut self, arr: &[T], size: usize, index: usize) {
        let available = self.max_length.saturating_sub(index);
        let copy_len = available.min(size).min(arr.len());
        if copy_len == 0 {
            return;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                arr.as_ptr(),
                (self.get_pointer() as *mut T).add(index),
                copy_len,
            );
        }
    }

    /// Fills the array with a value
    ///
    /// # Arguments
    /// * `value` - The value to fill the array with
    pub fn fill(&mut self, value: T) {
        unsafe {
            std::slice::from_raw_parts_mut(self.get_pointer() as *mut T, self.max_length)
                .fill(value);
        }
    }

    /// Removes an element at the specified index
    ///
    /// Shifts subsequent elements to the left in place.
    ///
    /// IL2CPP arrays are fixed-size, so this does not change `max_length`.
    /// The final slot is left as a duplicate of the former tail element.
    ///
    /// # Arguments
    /// * `index` - The index of the element to remove
    pub fn remove_at(&mut self, index: usize) {
        if index >= self.max_length {
            return;
        }

        if index + 1 < self.max_length {
            unsafe {
                let data = self.get_pointer() as *mut T;
                std::ptr::copy(
                    data.add(index + 1),
                    data.add(index),
                    self.max_length - index - 1,
                );
            }
        }
    }

    /// Removes a range of elements
    ///
    /// Shifts subsequent elements to the left in place.
    ///
    /// IL2CPP arrays are fixed-size, so this does not change `max_length`.
    /// Vacated tail slots are left duplicated from the previous tail values.
    ///
    /// # Arguments
    /// * `index` - The starting index
    /// * `count` - The number of elements to remove
    pub fn remove_range(&mut self, index: usize, count: usize) {
        if index >= self.max_length {
            return;
        }

        if count == 0 {
            return;
        }

        let count = count.min(self.max_length - index);
        let tail_len = self.max_length - index - count;

        if tail_len > 0 {
            unsafe {
                let data = self.get_pointer() as *mut T;
                std::ptr::copy(data.add(index + count), data.add(index), tail_len);
            }
        }
    }

    /// Removes all elements from the array
    ///
    /// IL2CPP arrays are fixed-size managed objects, so clearing them by
    /// shrinking the header is unsupported. Overwrite elements explicitly if
    /// you need sentinel values.
    pub fn remove_all(&mut self) {}

    /// Converts the array to a Rust Vec
    ///
    /// # Returns
    /// * `Vec<T>` - A new vector containing the array elements
    pub fn to_vector(&self) -> Vec<T> {
        unsafe { std::slice::from_raw_parts(self.get_pointer(), self.max_length).to_vec() }
    }

    /// Creates a new array instance
    ///
    /// # Arguments
    /// * `class` - The class of the elements
    /// * `size` - The size of the array
    ///
    /// # Returns
    /// * `*mut Self` - A pointer to the new array
    pub fn new(class: &Class, size: usize) -> *mut Self {
        unsafe { api::array_new(class.address, size as u32) as *mut Self }
    }
}
