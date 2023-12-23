#[derive(Debug)]
pub struct Texture {
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) data: Vec<u8>,
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
