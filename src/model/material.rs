use std::{fmt::Debug, mem::MaybeUninit};

use citro3d::{
    math::{FVec3, FVec4},
    Instance,
};
use ctru::linear::LinearAllocator;

use crate::Uniforms;

use super::{colour::Colour, texture::Texture};

#[doc(alias = "GPU_TEXCOLOR")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureColour {
    /// 8-bit Red + 8-bit Green + 8-bit Blue + 8-bit Alpha
    Rgba8 = ctru_sys::GPU_RGBA8,
    /// 8-bit Red + 8-bit Green + 8-bit Blue
    Rgb8 = ctru_sys::GPU_RGB8,
    /// 5-bit Red + 5-bit Green + 5-bit Blue + 1-bit Alpha
    Rgba5551 = ctru_sys::GPU_RGBA5551,
    /// 5-bit Red + 6-bit Green + 5-bit Blue
    Rgb565 = ctru_sys::GPU_RGB565,
    /// 4-bit Red + 4-bit Green + 4-bit Blue + 4-bit Alpha
    Rgba4 = ctru_sys::GPU_RGBA4,
    /// 8-bit Luminance + 8-bit Alpha
    La8 = ctru_sys::GPU_LA8,
    /// 8-bit Hi + 8-bit Lo
    HiLo8 = ctru_sys::GPU_HILO8,
    /// 8-bit Luminance
    L8 = ctru_sys::GPU_L8,
    /// 8-bit Alpha
    A8 = ctru_sys::GPU_A8,
    /// 4-bit Luminance + 4-bit Alpha
    La4 = ctru_sys::GPU_LA4,
    /// 4-bit Luminance
    L4 = ctru_sys::GPU_L4,
    /// 4-bit Alpha
    A4 = ctru_sys::GPU_A4,
    /// ETC1 texture compression
    Etc1 = ctru_sys::GPU_ETC1,
    /// ETC1 texture compression + 4-bit Alpha
    Etc1A4 = ctru_sys::GPU_ETC1A4,
}

impl TextureColour {
    fn size(&self) -> usize {
        match self {
            TextureColour::Rgba8 => 32,
            TextureColour::Rgb8 => 24,
            TextureColour::Rgba5551
            | TextureColour::Rgb565
            | TextureColour::Rgba4
            | TextureColour::La8
            | TextureColour::HiLo8 => 16,
            TextureColour::L8 | TextureColour::A8 | TextureColour::La4 | TextureColour::Etc1A4 => 8,
            TextureColour::L4 | TextureColour::A4 | TextureColour::Etc1 => 4,
        }
    }
}

impl TryFrom<ctru_sys::GPU_TEXCOLOR> for TextureColour {
    type Error = citro3d::Error;

    fn try_from(value: ctru_sys::GPU_TEXCOLOR) -> Result<Self, Self::Error> {
        match value {
            ctru_sys::GPU_RGBA8 => Ok(Self::Rgba8),
            ctru_sys::GPU_RGB8 => Ok(Self::Rgb8),
            ctru_sys::GPU_RGBA5551 => Ok(Self::Rgba5551),
            ctru_sys::GPU_RGB565 => Ok(Self::Rgb565),
            ctru_sys::GPU_RGBA4 => Ok(Self::Rgba4),
            ctru_sys::GPU_LA8 => Ok(Self::La8),
            ctru_sys::GPU_HILO8 => Ok(Self::HiLo8),
            ctru_sys::GPU_L8 => Ok(Self::L8),
            ctru_sys::GPU_A8 => Ok(Self::A8),
            ctru_sys::GPU_LA4 => Ok(Self::La4),
            ctru_sys::GPU_L4 => Ok(Self::L4),
            ctru_sys::GPU_A4 => Ok(Self::A4),
            ctru_sys::GPU_ETC1 => Ok(Self::Etc1),
            ctru_sys::GPU_ETC1A4 => Ok(Self::Etc1A4),
            _ => Err(citro3d::Error::NotFound),
        }
    }
}

#[doc(alias = "GPU_TEXTURE_FILTER_PARAM")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureFilterParam {
    /// Nearest-neighbor interpolation.
    Nearest = ctru_sys::GPU_NEAREST,
    /// Linear interpolation.
    Linear = ctru_sys::GPU_LINEAR,
}

#[doc(alias = "GPU_TEXTURE_WRAP_PARAM")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureWrapParam {
    /// Clamps to edge.
    ClampToEdge = ctru_sys::GPU_CLAMP_TO_EDGE,
    /// Clamps to border.
    ClampToBorder = ctru_sys::GPU_CLAMP_TO_BORDER,
    /// Repeats texture.
    Repeat = ctru_sys::GPU_REPEAT,
    /// Repeats with mirrored texture.
    MirroredRepeat = ctru_sys::GPU_MIRRORED_REPEAT,
}

pub struct C3DTex(Box<citro3d_sys::C3D_Tex, LinearAllocator>);

impl C3DTex {
    #[doc(alias = "C3D_TexInit")]
    pub fn new(width: u16, height: u16, format: TextureColour) -> citro3d::Result<Self> {
        let mut raw = Box::<citro3d_sys::C3D_Tex, LinearAllocator>::new_zeroed_in(LinearAllocator);
        let raw = unsafe {
            if !citro3d_sys::C3D_TexInit(raw.as_mut_ptr(), width, height, format as _) {
                return Err(citro3d::Error::FailedToInitialize);
            }
            raw.assume_init()
        };
        Ok(Self(raw))
    }

    pub fn dims(&self) -> (u16, u16) {
        unsafe {
            (
                self.0.__bindgen_anon_2.__bindgen_anon_1.width,
                self.0.__bindgen_anon_2.__bindgen_anon_1.height,
            )
        }
    }

    #[doc(alias = "C3D_TexBind")]
    pub fn bind(&self, unit_id: i32) {
        unsafe { citro3d_sys::C3D_TexBind(unit_id, self.as_raw().cast_mut()) }
    }

    #[doc(alias = "C3D_TexUpload")]
    pub fn upload<T: AsRef<[u8]>>(&self, data: T) {
        let buf = data.as_ref();

        let (width, height) = self.dims();
        let (width, height) = (width as usize, height as usize);
        assert!(
            buf.len()
                >= width
                    * height
                    * TextureColour::try_from(self.0.fmt())
                        .expect("unknown texture colour type")
                        .size()
                    / 8
        );

        unsafe { citro3d_sys::C3D_TexUpload(self.as_raw().cast_mut(), buf.as_ptr().cast()) }
    }

    #[doc(alias = "C3D_TexSetFilter")]
    pub fn set_filter(&self, mag_filter: TextureFilterParam, min_filter: TextureFilterParam) {
        unsafe {
            citro3d_sys::C3D_TexSetFilter(
                self.as_raw().cast_mut(),
                mag_filter as u32,
                min_filter as u32,
            )
        }
    }

    #[doc(alias = "C3D_TexSetWrap")]
    pub fn set_wrap(&self, wrap_s: TextureWrapParam, wrap_t: TextureWrapParam) {
        unsafe {
            citro3d_sys::C3D_TexSetWrap(self.as_raw().cast_mut(), wrap_s as u32, wrap_t as u32)
        }
    }

    pub fn as_raw(&self) -> *const citro3d_sys::C3D_Tex {
        &*self.0
    }
}

impl Debug for C3DTex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (width, height) = self.dims();
        write!(f, "C3DTex({}\u{00D7}{})", width, height)
    }
}

impl Drop for C3DTex {
    #[doc(alias = "C3D_TexDelete")]
    fn drop(&mut self) {
        unsafe { citro3d_sys::C3D_TexDelete(self.as_raw().cast_mut()) }
    }
}

#[derive(Debug, Default)]
pub struct Material {
    texture: Option<Texture>,
    colour: Option<Colour>,
    ambient: Option<Colour>,
    vertex_colours: bool,
    citro_tex: Option<C3DTex>,
}

impl Material {
    pub fn new(
        texture: Option<Texture>,
        colour: Option<Colour>,
        ambient: Option<Colour>,
        vertex_colours: bool,
    ) -> Self {
        let citro_tex = Self::make_texture(&texture);
        Self {
            texture,
            colour,
            ambient,
            vertex_colours,
            citro_tex,
        }
    }

    pub fn use_vertex_colours(&self) -> bool {
        self.vertex_colours
    }

    fn make_texture(texture: &Option<Texture>) -> Option<C3DTex> {
        if let Some(tex) = texture {
            let t = C3DTex::new(tex.width, tex.height, TextureColour::Rgba8).ok()?;
            t.upload(&tex.data);
            Some(t)
        } else {
            None
        }
    }

    pub fn get_texture(&self) -> Option<&C3DTex> {
        if let Some(tex) = &self.citro_tex {
            Some(tex)
        } else {
            None
        }
    }

    pub fn set_uniforms(&self, _gpu: &mut Instance, uniforms: &Uniforms) {
        let amb = if let Some(clr) = &self.ambient {
            clr.into()
        } else {
            FVec4::new(0.0, 0.0, 0.0, 0.0)
        };

        let emi = if let Some(clr) = &self.colour {
            clr.into()
        } else {
            FVec4::new(0.0, 0.0, 0.0, 0.0)
        };

        unsafe {
            citro3d_sys::C3D_FVUnifSet(
                citro3d::shader::Type::Vertex.into(),
                uniforms.material_ambient.into(),
                amb.x(),
                amb.y(),
                amb.z(),
                amb.w(),
            );
            citro3d_sys::C3D_FVUnifSet(
                citro3d::shader::Type::Vertex.into(),
                uniforms.material_emission.into(),
                emi.x(),
                emi.y(),
                emi.z(),
                emi.w(),
            );
        }
    }
}
