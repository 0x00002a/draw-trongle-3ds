use std::{fs::read, iter::repeat};

use crate::{
    model::{colour::Colour, material::Material, shape::Shape, texture::Texture, Model},
    Vec2, Vec3, Vert,
};

pub fn parse_obj(path: &str) -> Vec<Model<Vert>> {
    let mut obj = obj::Obj::load(path).unwrap();
    obj.load_mtls().unwrap();

    let vertices = obj
        .data
        .position
        .iter()
        .map(|e| Vec3 {
            x: e[0],
            y: e[1],
            z: e[2],
        })
        .collect::<Vec<_>>();

    let tex_coords = obj
        .data
        .texture
        .iter()
        .map(|e| Vec2 {
            x: e[0] * (480.0 / 512.0),
            y: (1.0 - e[1] * (395.0 / 512.0)),
        })
        .collect::<Vec<_>>();

    obj.data
        .objects
        .iter()
        .map(|e| {
            let shapes = e
                .groups
                .iter()
                .map(|g| {
                    let mat = &g.material;
                    let (col, tex) = if let Some(m) = mat {
                        match m {
                            obj::ObjMaterial::Ref(_) => todo!(),
                            obj::ObjMaterial::Mtl(m) => {
                                let col = m.kd.map(|rgb| {
                                    Colour::new(
                                        (rgb[0] * 255.0) as u8,
                                        (rgb[1] * 255.0) as u8,
                                        (rgb[2] * 255.0) as u8,
                                        0xFF,
                                    )
                                });

                                let tex = m
                                    .map_kd
                                    .as_ref()
                                    .map(|t| Texture::new(512, 512, read(t).unwrap()));

                                (col, tex)
                            }
                        }
                    } else {
                        (None, None)
                    };
                    let polys = g
                        .polys
                        .iter()
                        .flat_map(|p| {
                            let verts =
                                p.0.iter()
                                    .map(|i| vertices[i.0].clone())
                                    .take(3)
                                    .collect::<Vec<_>>();
                            let texs = p
                                .0
                                .iter()
                                .map(|i| i.1.map_or(Vec2::new(0.0, 0.0), |t| tex_coords[t].clone()))
                                .take(3)
                                .collect::<Vec<_>>();
                            verts
                                .into_iter()
                                .zip(texs)
                                .map(|(v, t)| Vert { pos: v, tex: t })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    Shape::new(
                        Material::new(
                            tex.or_else(|| {
                                Some(Texture::new(
                                    64,
                                    64,
                                    repeat(0).take(64 * 64 * 4).collect::<Vec<_>>(),
                                ))
                            }),
                            col,
                            None,
                            true,
                        ),
                        citro3d::buffer::Primitive::Triangles,
                        &polys,
                    )
                })
                .collect::<Vec<_>>();
            Model::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), shapes)
        })
        .collect::<_>()
}
