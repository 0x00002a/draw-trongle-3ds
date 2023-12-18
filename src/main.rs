#![feature(allocator_api)]
use citro3d::{
    attrib, buffer,
    macros::include_shader,
    math::{FVec3, FVec4, IVec, Matrix, Matrix4},
    render::{ClearFlags, Target},
    shader::{self, Program},
    texenv, Instance,
};
use ctru::{
    linear::LinearAllocator,
    prelude::*,
    services::gfx::{RawFrameBuffer, Screen},
};

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

#[repr(C)]
struct Vert {
    pos: Vec3,
    colour: Vec3,
}

fn vert_attrs() -> attrib::Info {
    let mut attrs = attrib::Info::new();
    let reg0 = attrib::Register::new(0).unwrap();
    let reg1 = attrib::Register::new(1).unwrap();
    attrs.add_loader(reg0, attrib::Format::Float, 3).unwrap();
    attrs.add_loader(reg1, attrib::Format::Float, 3).unwrap();
    attrs
}

const SHADER: &[u8] = include_shader!("../vshader.pica");

fn main() {
    let apt = Apt::new().unwrap();
    let mut hid = Hid::new().unwrap();
    let gfx = Gfx::new().unwrap();
    //let _console = Console::new(gfx.bottom_screen.borrow_mut());

    let mut gpu = Instance::new().expect("failed to init citro3d");
    let mut gpu_buf = buffer::Info::new();
    let verts = Box::new_in(
        [
            Vert {
                pos: Vec3::new(0., 0.5, -3.),
                colour: Vec3::new(1., 1., 1.),
            },
            Vert {
                pos: Vec3::new(-0.5, -0.5, -3.),
                colour: Vec3::new(1., 1., 1.),
            },
            Vert {
                pos: Vec3::new(0.5, -0.5, -3.),
                colour: Vec3::new(1., 1., 1.),
            },
        ],
        LinearAllocator,
    );
    let v_attrs = vert_attrs();
    let buf_vtos = gpu_buf
        .add(&*verts, &v_attrs)
        .expect("failed to bind verts");

    let shader_lib = shader::Library::from_bytes(SHADER).expect("failed to load shader");
    let vert_shader = shader_lib.get(0).unwrap();
    let vert_prog = Program::new(vert_shader).unwrap();
    gpu.bind_program(&vert_prog);

    let proj_uniform = vert_prog.get_uniform("projection").unwrap();

    let mut top_screen = gfx.top_screen.borrow_mut();
    let RawFrameBuffer { width, height, .. } = top_screen.raw_framebuffer();

    let mut target = Target::new(width, height, top_screen, None).unwrap();

    //println!("Hello, World!");
    //println!("\x1b[29;16HPress Start to exit");
    let stage0 = texenv::Stage::new(0).unwrap();
    gpu.texenv(stage0)
        .src(texenv::Mode::BOTH, texenv::Source::PrimaryColor, None, None)
        .func(texenv::Mode::BOTH, texenv::CombineFunc::Replace);

    while apt.main_loop() {
        gfx.wait_for_vblank();

        gpu.render_frame_with(|gpu| {
            target.clear(ClearFlags::ALL, 0, 0);
            gpu.select_render_target(&target).unwrap();
            gpu.bind_vertex_uniform(proj_uniform, &Matrix4::identity());
            gpu.set_attr_info(&v_attrs);
            gpu.draw_arrays(buffer::Primitive::Triangles, buf_vtos);
        });

        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }
    }
}
