use std::{fmt::Debug, mem::MaybeUninit};

use citro3d::{
    math::{FVec3, FVec4},
    texture::{Tex, TexParams},
    Instance,
};
use ctru::linear::LinearAllocator;

use crate::Uniforms;

use super::{colour::Colour, texture::Texture};

#[derive(Debug, Default)]
pub struct Material {
    texture: Option<Texture>,
    colour: Option<Colour>,
    ambient: Option<Colour>,
    vertex_colours: bool,
    citro_tex: Option<Tex>,
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

    fn make_texture(texture: &Option<Texture>) -> Option<Tex> {
        if let Some(tex) = texture {
            let t = Tex::new(TexParams::new_2d(tex.width, tex.height)).ok()?;
            t.upload(&tex.data);
            Some(t)
        } else {
            None
        }
    }

    pub fn get_texture(&self) -> Option<&Tex> {
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
