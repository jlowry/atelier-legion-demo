use std::ops::{Deref, DerefMut};

// For now just wrap the input helper that skulpin provides
pub struct ImguiResource {
    pub imgui_manager: skulpin::ImguiManager,
}

impl ImguiResource {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new(imgui_manager: skulpin::ImguiManager) -> Self {
        ImguiResource { imgui_manager }
    }
}

impl Deref for ImguiResource {
    type Target = skulpin::ImguiManager;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.imgui_manager
    }
}

impl DerefMut for ImguiResource {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.imgui_manager
    }
}
