#![feature(allocator_api)]
#![feature(new_uninit)]

use std::{f32::consts::TAU, iter::repeat, mem::MaybeUninit, time::Duration};

use citro3d::{
    attrib::{self, Format},
    buffer,
    macros::include_shader,
    math::{
        AspectRatio, ClipPlanes, FVec3, FVec4, IVec, Matrix, Matrix4, Projection,
        StereoDisplacement,
    },
    render::{self, ClearFlags, DepthFormat::Depth16, Target},
    shader::{self, Program},
    texenv,
    uniform::Index,
    Instance,
};
use ctru::{
    error::ResultCode,
    linear::LinearAllocator,
    prelude::*,
    services::{
        fs::Fs,
        gfx::{RawFrameBuffer, Screen, TopScreen3D},
        ir_user::{CirclePadProInputResponse, ConnectionStatus, IrUser},
        romfs::RomFS,
        svc::HandleExt,
    },
};
use ctru_sys::Handle;
use include_texture_macro::include_texture;
use model::{material::Material, shape::Shape, texture::Texture, Model};
use vert_attr::{VertAttrBuilder, VertAttrs};

use crate::{model::colour::Colour, obj::parse_obj};

const DEADZONE: f32 = 0.01;
const CIRCLE_DEADZONE: f32 = 15.0;

mod model;
mod obj;

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

#[derive(VertAttrBuilder, Clone, Debug)]
#[repr(C)]
struct Vert {
    pos: Vec3,
    tex: Vec2,
}

const SHADER: &[u8] = include_shader!("../shader.pica");

const BOWSER: &[u8] = include_texture!("../bowser.png");
const PEACH: &[u8] = include_texture!("../peach.png");

const PACKET_INFO_SIZE: usize = 8;
const MAX_PACKET_SIZE: usize = 32;
const PACKET_COUNT: usize = 1;
const PACKET_BUFFER_SIZE: usize = PACKET_COUNT * (PACKET_INFO_SIZE + MAX_PACKET_SIZE);
const CPP_CONNECTION_POLLING_PERIOD_MS: u8 = 0x08;
const CPP_POLLING_PERIOD_MS: u8 = 0x32;

struct CirclePadPro {
    ir_user: IrUser,
    connection_status_event: Handle,
    receive_packet_event: Handle,
    last_response: Option<CirclePadProInputResponse>,
}

impl CirclePadPro {
    pub fn new() -> ctru::Result<Self> {
        let ir_user = IrUser::init(
            PACKET_BUFFER_SIZE,
            PACKET_COUNT,
            PACKET_BUFFER_SIZE,
            PACKET_COUNT,
        )?;

        let connection_status_event = ir_user.get_connection_status_event()?;
        let receive_packet_event = ir_user.get_recv_event()?;

        Ok(Self {
            ir_user,
            connection_status_event,
            receive_packet_event,
            last_response: None,
        })
    }

    pub fn get_input(&self) -> Option<&CirclePadProInputResponse> {
        match &self.last_response {
            Some(r) => Some(&r),
            None => None,
        }
    }

    pub fn connect(&mut self) -> ctru::Result<()> {
        loop {
            self.ir_user
                .require_connection(ctru::services::ir_user::IrDeviceId::CirclePadPro)?;

            if let Err(e) = self
                .connection_status_event
                .wait_for_event(Duration::from_millis(100))
            {
                if !e.is_timeout() {
                    return Err(e);
                }
            }

            if self.ir_user.get_status_info().connection_status == ConnectionStatus::Connected {
                println!("Connected");
                break;
            }

            self.ir_user.disconnect().unwrap();

            if let Err(e) = self
                .connection_status_event
                .wait_for_event(Duration::from_millis(100))
            {
                if !e.is_timeout() {
                    return Err(e);
                }
            }
        }

        loop {
            if let Err(e) = self
                .ir_user
                .request_input_polling(CPP_CONNECTION_POLLING_PERIOD_MS)
            {
                println!("Error: {e:?}");
            }

            let recv_event_result = self
                .receive_packet_event
                .wait_for_event(Duration::from_millis(100));

            if recv_event_result.is_ok() {
                println!("Got first packet from CPP");
                self.handle_packets();
                break;
            }
        }

        Ok(())
    }

    pub fn handle_packets(&mut self) {
        let packets = self.ir_user.get_packets().unwrap();
        let packet_count = packets.len();
        let Some(last_packet) = packets.last() else {
            return;
        };

        let cpp_response = CirclePadProInputResponse::try_from(last_packet).unwrap();
        self.last_response = Some(cpp_response);

        self.ir_user
            .release_received_data(packet_count as u32)
            .unwrap();
        if let Err(e) = self.ir_user.request_input_polling(CPP_POLLING_PERIOD_MS) {
            println!("Error: {e:?}");
        }
    }

    pub fn scan_input(&mut self) {
        let packet_received = self
            .receive_packet_event
            .wait_for_event(Duration::ZERO)
            .is_ok();
        if packet_received {
            self.handle_packets();
        }
    }
}

#[no_mangle]
unsafe extern "C" fn __appInit() {
    let _ = ctru_sys::srvInit();
}

#[no_mangle]
unsafe extern "C" fn hidShouldUseIrrst() -> bool {
    false
}

extern "C" {
    static irrstHandle: u32;
    static irrstMemHandle: u32;
    static irrstEvent: u32;

    static irrstSharedMem: *mut u32;
}

pub struct Uniforms {
    pub model_matrix: Index,
    pub camera_matrix: Index,
    pub projection_matrix: Index,
    pub light_colour: Index,
    pub material_emission: Index,
    pub material_ambient: Index,
    pub material_diffuse: Index,
    pub material_specular: Index,
}

fn main() {
    let apt = Apt::new().unwrap();
    let _fs = Fs::new().unwrap();
    let gfx = Gfx::new().unwrap();
    let _console = Console::new(gfx.bottom_screen.borrow_mut());
    let mut soc = Soc::new().unwrap();
    // will use `tty` if this fails
    let _ = soc.redirect_to_3dslink(true, true);
    let _romfs = RomFS::new().unwrap();

    //let mut cpp = CirclePadPro::new().unwrap();

    let mut hid = Hid::new().unwrap();

    //cpp.connect().unwrap();

    unsafe {
        println!("irrstHandle: {irrstHandle:08X}\nirrstMemHandle: {irrstMemHandle:08X}\nirrstEvent: {irrstEvent:08X}\nirrstSharedMem: {irrstSharedMem:?}")
    };

    let mut gpu = Instance::new().expect("failed to init citro3d");

    hid.set_gyroscope(true).unwrap();

    let coeff = {
        let mut coeff = MaybeUninit::uninit();
        unsafe {
            let r = ResultCode(ctru_sys::HIDUSER_GetGyroscopeRawToDpsCoefficient(
                coeff.as_mut_ptr(),
            ));
            println!("{:?}", r);
            coeff.assume_init()
        }
    };

    println!("coeff: {coeff}");

    let top_screen = TopScreen3D::from(&gfx.top_screen);

    let (mut top_screen_left, mut top_screen_right) = top_screen.split_mut();

    let RawFrameBuffer { width, height, .. } = top_screen_left.raw_framebuffer();
    let mut top_left_target = render::Target::new(width, height, top_screen_left, Some(Depth16))
        .expect("failed to create left render target");

    let RawFrameBuffer { width, height, .. } = top_screen_right.raw_framebuffer();
    let mut top_right_target = render::Target::new(width, height, top_screen_right, Some(Depth16))
        .expect("failed to create right render target");

    let shader_lib = shader::Library::from_bytes(SHADER).expect("failed to load shader");
    let vert_shader = shader_lib.get(0).unwrap();
    let vert_prog = Program::new(vert_shader).unwrap();
    gpu.bind_program(&vert_prog);

    let model_uniform = vert_prog.get_uniform("modelMtx").unwrap();
    let cam_uniform = vert_prog.get_uniform("camMtx").unwrap();
    let proj_uniform = vert_prog.get_uniform("projMtx").unwrap();

    let light_uniform = vert_prog.get_uniform("lightClr").unwrap();

    let emi_uniform = vert_prog.get_uniform("mat_emi").unwrap();
    let amb_uniform = vert_prog.get_uniform("mat_amb").unwrap();
    let dif_uniform = vert_prog.get_uniform("mat_dif").unwrap();
    let spe_uniform = vert_prog.get_uniform("mat_spe").unwrap();

    let uniforms = Uniforms {
        model_matrix: model_uniform,
        camera_matrix: cam_uniform,
        projection_matrix: proj_uniform,
        light_colour: light_uniform,
        material_emission: emi_uniform,
        material_ambient: amb_uniform,
        material_diffuse: dif_uniform,
        material_specular: spe_uniform,
    };

    //println!("Hello, World!");
    //println!("\x1b[29;16HPress Start to exit");

    let mut cam_pos = Vec3::new(0.0, 0.0, 0.0);
    let mut cam_rot = Vec3::new(0.0, 0.0, 0.0);

    /*let mut mdl = Model::new(
        Vec3::new(0.0, 0.0, -1.5),
        Vec3::new(0.0, 0.0, 0.0),
        vec![
            Shape::new(
                Material::new(
                    Some(Texture::new(64, 64, BOWSER.to_vec())),
                    None,
                    None,
                    true,
                ),
                buffer::Primitive::TriangleFan,
                &[
                    Vert {
                        pos: Vec3::new(-0.5, 0.5, 0.0),
                        tex: Vec2::new(0.0, 1.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, -0.5, 0.0),
                        tex: Vec2::new(0.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, -0.5, 0.0),
                        tex: Vec2::new(1.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, 0.5, 0.0),
                        tex: Vec2::new(1.0, 1.0),
                    },
                ],
            ),
            Shape::new(
                Material::new(
                    Some(Texture::new(
                        64,
                        64,
                        repeat(0).take(64 * 64 * 4).collect::<Vec<_>>(),
                    )),
                    Some(Colour::new(0xFF, 0x00, 0xFF, 0xFF)),
                    None,
                    false,
                ),
                buffer::Primitive::TriangleFan,
                &[
                    Vert {
                        pos: Vec3::new(0.5, 0.5, 0.0),
                        tex: Vec2::new(0.0, 1.0),
                    },
                    Vert {
                        pos: Vec3::new(0.5, -0.5, 0.0),
                        tex: Vec2::new(0.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, -0.5, 0.0),
                        tex: Vec2::new(1.0, 0.0),
                    },
                    Vert {
                        pos: Vec3::new(-0.5, 0.5, 0.0),
                        tex: Vec2::new(1.0, 1.0),
                    },
                ],
            ),
        ],
    );*/
    let models = parse_obj("romfs:/textured-cornell-box.obj");
    for i in &models {
        println!("{:#?}", i);
    }

    while apt.main_loop() {
        gfx.wait_for_vblank();

        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        let (x, y) = hid.circlepad_position();
        let (x, y) = (x as f32, y as f32);
        //println!("{x}, {y}");
        if x.abs() > CIRCLE_DEADZONE {
            cam_pos.x -= x / 1000.0
        }
        if y.abs() > CIRCLE_DEADZONE {
            cam_pos.z += y / 1000.0
        }
        if hid.keys_held().contains(KeyPad::X) {
            cam_pos.y -= 0.01;
        }
        if hid.keys_held().contains(KeyPad::Y) {
            cam_pos.y += 0.01;
        }

        /*if hid.keys_down().contains(KeyPad::R) {
            mdl.rot.z -= 0.25;
            mdl.rot.z %= TAU;
        }
        if hid.keys_down().contains(KeyPad::L) {
            mdl.rot.z += 0.25;
            mdl.rot.z %= TAU;
        }*/

        let (roll, pitch, yaw) = hid.gyroscope_rate().unwrap().into();
        let (roll, pitch, yaw) = (
            roll as f32 / (coeff * 128.0 * TAU),
            pitch as f32 / (coeff * 128.0 * TAU),
            yaw as f32 / (coeff * 128.0 * TAU),
        );

        if hid.keys_held().contains(KeyPad::A) {
            if roll.abs() > DEADZONE {
                cam_rot.x += roll;
                cam_rot.x %= TAU;
            }

            if pitch.abs() > DEADZONE {
                cam_rot.y -= pitch;
                cam_rot.y %= TAU;
            }

            if yaw.abs() > DEADZONE {
                cam_rot.z -= yaw;
                cam_rot.z %= TAU;
            }
        }

        /*cpp.scan_input();
        let cpp_input = cpp.get_input();
        if let Some(input) = cpp_input {
            let (x, y) = (
                input.c_stick_x as f32 / 10.0 - 200.0,
                input.c_stick_y as f32 / 10.0 - 200.0,
            );
            //println!("c: {x}, {y}");
            if x.abs() > CIRCLE_DEADZONE {
                cam_rot.y += x / 1000.0
            }
            if y.abs() > CIRCLE_DEADZONE {
                cam_rot.x -= y / 1000.0
            }
        }*/

        gpu.render_frame_with(|inst| {
            let mut camera_matrix = Matrix4::identity();

            camera_matrix.translate(cam_pos.x, cam_pos.y, cam_pos.z);
            camera_matrix.rotate_x(cam_rot.x);
            camera_matrix.rotate_y(cam_rot.y);
            camera_matrix.rotate_z(cam_rot.z);

            inst.bind_vertex_uniform(uniforms.camera_matrix, &camera_matrix);

            let mut render_to = |target: &mut render::Target, projection| {
                target.clear(ClearFlags::ALL, 0, 0);
                inst.select_render_target(target).unwrap();

                inst.bind_vertex_uniform(uniforms.projection_matrix, projection);
                /*gpu.set_attr_info(&v_attrs);
                gpu.draw_arrays(buffer::Primitive::TriangleFan, buf_vtos);*/
                //mdl.draw(inst, &uniforms);
                for mdl in &models {
                    mdl.draw(inst, &uniforms);
                }
            };

            let Projections {
                left_eye,
                right_eye,
                ..
            } = calculate_projections();

            render_to(&mut top_left_target, &left_eye);
            render_to(&mut top_right_target, &right_eye);
        });

        //println!("{:?}", hid.gyroscope_rate().unwrap());
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
