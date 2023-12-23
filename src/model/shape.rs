use super::material::Material;
use citro3d::{
    attrib,
    buffer::{self, Primitive},
    Instance,
};
use ctru::linear::LinearAllocator;
use vert_attr::VertAttrBuilder;

#[derive(Debug)]
pub struct Shape<T: VertAttrBuilder + Clone> {
    mat: Material,
    prim_type: Primitive,
    verts: Vec<T, LinearAllocator>,
    attr_info: attrib::Info,
}

impl<T: VertAttrBuilder + Clone> Shape<T> {
    pub fn new(mat: Material, prim_type: Primitive, verts: &[T]) -> Self {
        let mut vertex_buffer = Vec::with_capacity_in(verts.len(), LinearAllocator);
        vertex_buffer.extend_from_slice(verts);

        let attr_info = T::vert_attrs();

        Self {
            mat,
            prim_type,
            verts: vertex_buffer,
            attr_info,
        }
    }

    pub fn draw(&self, gpu: &mut Instance) {
        let tex = self.mat.make_texture();

        let stage0 = citro3d::texenv::Stage::new(0).unwrap();

        if let Some(t) = &tex {
            t.bind(0);

            if self.mat.use_vertex_colours() {
                gpu.texenv(stage0)
                    .src(
                        citro3d::texenv::Mode::BOTH,
                        citro3d::texenv::Source::Texture0,
                        Some(citro3d::texenv::Source::PrimaryColor),
                        None,
                    )
                    .func(
                        citro3d::texenv::Mode::BOTH,
                        citro3d::texenv::CombineFunc::Modulate,
                    );
            } else {
                gpu.texenv(stage0)
                    .src(
                        citro3d::texenv::Mode::BOTH,
                        citro3d::texenv::Source::Texture0,
                        None,
                        None,
                    )
                    .func(
                        citro3d::texenv::Mode::BOTH,
                        citro3d::texenv::CombineFunc::Replace,
                    );
            }
        } else {
            let env = gpu.texenv(stage0);
            env.reset();

            env.src(
                citro3d::texenv::Mode::BOTH,
                citro3d::texenv::Source::PrimaryColor,
                None,
                None,
            )
            .func(
                citro3d::texenv::Mode::BOTH,
                citro3d::texenv::CombineFunc::Replace,
            );
        }

        let mut buf_info = buffer::Info::new();
        let buf_vtos = buf_info
            .add(&self.verts, &self.attr_info)
            .expect("failed to bind verts");

        gpu.set_attr_info(&self.attr_info);
        gpu.draw_arrays(self.prim_type, buf_vtos);
    }
}
