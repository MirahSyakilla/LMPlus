/// Reads a value of type `T` from the given memory address.
pub unsafe fn read<T: Copy>(addr: usize) -> Result<T, String> {
    if addr == 0 {
        return Err("null pointer read".to_string());
    }
    Ok(std::ptr::read(addr as *const T))
}

/// Writes a value of type `T` to the given memory address.
pub unsafe fn write<T: Copy>(addr: usize, value: T) -> Result<(), String> {
    if addr == 0 {
        return Err("null pointer write".to_string());
    }
    std::ptr::write(addr as *mut T, value);
    Ok(())
}
