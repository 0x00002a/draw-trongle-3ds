use citro3d::{math::Matrix4, uniform::Index, Instance};
use vert_attr::VertAttrBuilder;

use crate::{Uniforms, Vec3};

use self::shape::Shape;

pub mod colour;
pub mod material;
pub mod shape;
pub mod texture;

#[derive(Debug)]
pub struct Model<T: VertAttrBuilder + Clone> {
    pub pos: Vec3,
    pub rot: Vec3,
    shapes: Vec<Shape<T>>,
}

impl<T: VertAttrBuilder + Clone> Model<T> {
    pub fn new(pos: Vec3, rot: Vec3, shapes: Vec<Shape<T>>) -> Self {
        Self { pos, rot, shapes }
    }

    pub fn draw(&self, gpu: &mut Instance, uniforms: &Uniforms) {
        let Vec3 { x, y, z } = self.pos;

        let mut transform = Matrix4::identity();

        transform.scale(1.0, 1.0, 1.0);

        transform.rotate_x(-self.rot.y);
        transform.rotate_y(self.rot.x);
        transform.rotate_z(self.rot.z);

        transform.translate(x, y, z);

        gpu.bind_vertex_uniform(uniforms.model_matrix, &transform);

        for shape in &self.shapes {
            shape.draw(gpu, uniforms);
        }
    }
}
