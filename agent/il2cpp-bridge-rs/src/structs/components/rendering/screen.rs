//! Unity Screen utility wrapper
use crate::api::cache;

pub struct Screen;

impl Screen {
    /// Gets the current screen width
    ///
    /// # Returns
    /// * `Result<i32, String>` - The width of the screen window
    pub fn get_width() -> Result<i32, String> {
        let class = cache::coremodule()
            .class("UnityEngine.Screen")
            .ok_or("Class 'UnityEngine.Screen' not found")?;

        unsafe {
            let method = class
                .method("get_width")
                .ok_or("Method 'get_width' not found")?;
            method.call::<i32>(&[])
        }
    }

    /// Gets the current screen height
    ///
    /// # Returns
    /// * `Result<i32, String>` - The height of the screen window
    pub fn get_height() -> Result<i32, String> {
        let class = cache::coremodule()
            .class("UnityEngine.Screen")
            .ok_or("Class 'UnityEngine.Screen' not found")?;

        unsafe {
            let method = class
                .method("get_height")
                .ok_or("Method 'get_height' not found")?;
            method.call::<i32>(&[])
        }
    }
}
