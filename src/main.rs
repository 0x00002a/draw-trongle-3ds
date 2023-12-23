#![feature(allocator_api)]
#![feature(new_uninit)]

use std::f32::consts::TAU;

use citro3d::{
    attrib::{self, Format},
    buffer,
    macros::include_shader,
    math::{
        AspectRatio, ClipPlanes, FVec3, FVec4, IVec, Matrix, Matrix4, Projection,
        StereoDisplacement,
    },
    render::{self, ClearFlags, Target},
    shader::{self, Program},
    texenv, Instance,
};
use ctru::{
    linear::LinearAllocator,
    prelude::*,
    services::gfx::{RawFrameBuffer, Screen, TopScreen3D},
};
use include_texture_macro::include_texture;
use model::{material::Material, shape::Shape, texture::Texture, Model};
use vert_attr::{VertAttrBuilder, VertAttrs};

const DEADZONE: i16 = 8;

mod model;

#[derive(Debug, Clone)]
#[repr(C)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl VertAttrs for Vec3 {
    const FORMAT: Format = Format::Float;
    const SIZE: u8 = 3;
}

#[derive(Debug, Clone)]
#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl VertAttrs for Vec2 {
    const FORMAT: Format = Format::Float;
    const SIZE: u8 = 2;
}

#[derive(VertAttrBuilder, Clone)]
#[repr(C)]
struct Vert {
    pos: Vec3,
    col: Vec3,
    tex: Vec2,
}

const SHADER: &[u8] = include_shader!("../shader.pica");

/*const BOWSER: &[u8] = include_texture!("../bowser.png");
const PEACH: &[u8] = include_texture!("../peach.png");*/
const COMBINED: &[u8] = include_texture!("../combined.png");

fn main() {
    let apt = Apt::new().unwrap();
    let mut hid = Hid::new().unwrap();
    let gfx = Gfx::new().unwrap();
    let _console = Console::new(gfx.bottom_screen.borrow_mut());

    let top_screen = TopScreen3D::from(&gfx.top_screen);

    let (mut top_screen_left, mut top_screen_right) = top_screen.split_mut();

    let RawFrameBuffer { width, height, .. } = top_screen_left.raw_framebuffer();
    let mut top_left_target = render::Target::new(width, height, top_screen_left, None)
        .expect("failed to create left render target");

    let RawFrameBuffer { width, height, .. } = top_screen_right.raw_framebuffer();
    let mut top_right_target = render::Target::new(width, height, top_screen_right, None)
        .expect("failed to create right render target");

    let mut gpu = Instance::new().expect("failed to init citro3d");

    let shader_lib = shader::Library::from_bytes(SHADER).expect("failed to load shader");
    let vert_shader = shader_lib.get(0).unwrap();
    let vert_prog = Program::new(vert_shader).unwrap();
    gpu.bind_program(&vert_prog);

    let proj_uniform = vert_prog.get_uniform("projMtx").unwrap();
    let model_uniform = vert_prog.get_uniform("modelMtx").unwrap();

    //println!("Hello, World!");
    //println!("\x1b[29;16HPress Start to exit");

    let mut mdl = Model::new(
        Vec3::new(0.0, 0.0, -1.5),
        Vec3::new(0.0, 0.0, 0.0),
        vec![
            Shape::new(
                Material::new(
                    Some(Texture::new(128, 64, COMBINED.to_vec())),
                    None,
                    None,
                    false,
                ),
                buffer::Primitive::TriangleFan,
                &[
                    Vert {
                        pos: Vec3::new(-0.5, 0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.0, 1.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, -0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, -0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.5, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, 0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.5, 1.0),
                    },
                ],
            ),
            Shape::new(
                Material::new(
                    Some(Texture::new(128, 64, COMBINED.to_vec())),
                    None,
                    None,
                    false,
                ),
                buffer::Primitive::TriangleFan,
                &[
                    Vert {
                        pos: Vec3::new(0.5, 0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.5, 1.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, -0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(0.5, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, -0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(1.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, 0.5, 0.0),
                        col: Vec3::new(0.0, 0.0, 0.0),
                        tex: Vec2::new(1.0, 1.0),
                    },
                ],
            ),
        ],
    );

    while apt.main_loop() {
        gfx.wait_for_vblank();

        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        let (x, y) = hid.circlepad_position();
        if x.abs() > DEADZONE {
            mdl.rot.x += (x as f32) / (154.0 * 4.0);
            mdl.rot.x %= TAU;
        }
        if y.abs() > DEADZONE {
            mdl.rot.y += (y as f32) / (154.0 * 4.0);
            mdl.rot.y %= TAU;
        }

        if hid.keys_down().contains(KeyPad::R) {
            mdl.rot.z -= 0.25;
            mdl.rot.z %= TAU;
        }
        if hid.keys_down().contains(KeyPad::L) {
            mdl.rot.z += 0.25;
            mdl.rot.z %= TAU;
        }

        gpu.render_frame_with(|inst| {
            let mut render_to = |target: &mut render::Target, projection| {
                target.clear(ClearFlags::ALL, 0, 0);
                inst.select_render_target(target).unwrap();
                inst.bind_vertex_uniform(proj_uniform, projection);
                /*gpu.set_attr_info(&v_attrs);
                gpu.draw_arrays(buffer::Primitive::TriangleFan, buf_vtos);*/
                mdl.draw(inst, Some(model_uniform));
            };

            let Projections {
                left_eye,
                right_eye,
                ..
            } = calculate_projections();

            render_to(&mut top_left_target, &left_eye);
            render_to(&mut top_right_target, &right_eye);
        });
    }
}

#[derive(Debug)]
struct Projections {
    left_eye: Matrix4,
    right_eye: Matrix4,
    center: Matrix4,
}

fn calculate_projections() -> Projections {
    // TODO: it would be cool to allow playing around with these parameters on
    // the fly with D-pad, etc.
    let slider_val = ctru::os::current_3d_slider_state();
    let interocular_distance = slider_val / 2.0;

    let vertical_fov = 40.0_f32.to_radians();
    let screen_depth = 2.0;

    let clip_planes = ClipPlanes {
        near: 0.01,
        far: 100.0,
    };

    let (left, right) = StereoDisplacement::new(interocular_distance, screen_depth);

    let (left_eye, right_eye) =
        Projection::perspective(vertical_fov, AspectRatio::TopScreen, clip_planes)
            .stereo_matrices(left, right);

    let center =
        Projection::perspective(vertical_fov, AspectRatio::BottomScreen, clip_planes).into();

    Projections {
        left_eye,
        right_eye,
        center,
    }
}
