use citro3d::math::FVec4;

#[derive(Debug)]
pub struct Colour([u8; 4]);

impl Colour {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    pub fn r(&self) -> u8 {
        self.0[0]
    }

    pub fn g(&self) -> u8 {
        self.0[1]
    }

    pub fn b(&self) -> u8 {
        self.0[2]
    }

    pub fn a(&self) -> u8 {
        self.0[3]
    }
}

impl Into<FVec4> for &Colour {
    fn into(self) -> FVec4 {
        let [r, g, b, a] = self.0;
        let (r, g, b, a) = (
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        );
        FVec4::new(r, g, b, a)
    }
}
