pub struct Texture {
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) data: Vec<u8>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl Texture {
    pub fn new(width: u16, height: u16, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }
}
