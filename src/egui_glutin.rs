use egui::{Modifiers, RawInput};
use glutin::{ContextWrapper, PossiblyCurrent};
use glutin::event_loop::ControlFlow;
use glutin::event::*;
use std::{ffi::{CString, c_void}, ptr, str::from_utf8};

pub struct EguiState
{
    pub windowed_context: ContextWrapper<PossiblyCurrent, glutin::window::Window>,

    pub ctx: egui::Context,
    pub pos_in_points: Option<egui::Pos2>,
    pub raw_input: RawInput,

    vao: u32,
    vbo: u32,
    pub tex: u32,
    shader: u32,

    buffer_size: u32,
}

pub fn paint_egui(clipped_meshes: Vec<egui::ClippedMesh>, egui_state: &mut EguiState)
{
    //todo: pass in window size
    unsafe
    {
        gl::Scissor(0, 0, 1024, 768);
        gl::ClearColor(0.0, 0.1, 0.2, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::STENCIL_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

        gl::BindVertexArray(egui_state.vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, egui_state.vbo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, egui_state.vbo);
        gl::UseProgram(egui_state.shader);
        gl::BindTextureUnit(0, egui_state.tex);
        gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
    }

    for clipped_mesh in &clipped_meshes
    {
        unsafe
        {
            let x = clipped_mesh.0.min.x.clamp(0.0, 1024.0);
            let y = clipped_mesh.0.min.y.clamp(0.0, 768.0);
            let width = clipped_mesh.0.max.x.clamp(x, 1024.0) as i32;
            let height = clipped_mesh.0.max.y.clamp(y, 768.0) as i32;
            gl::Scissor(x as i32, 768 - height, width - x as i32, height - y as i32);

            let buffer_size = ((clipped_mesh.1.indices.len() + (clipped_mesh.1.vertices.len() * 5)) * 4) as u32;

            if egui_state.buffer_size < buffer_size
            {
                gl::NamedBufferData
                (
                    egui_state.vbo,
                    buffer_size as isize,
                    ptr::null(),
                    gl::DYNAMIC_DRAW,
                );

                egui_state.buffer_size = buffer_size;
            }

            gl::NamedBufferSubData
            (
                egui_state.vbo,
                0,
                clipped_mesh.1.indices.len() as isize * 4,
                clipped_mesh.1.indices.as_ptr() as *const c_void,
            );

            gl::NamedBufferSubData
            (
                egui_state.vbo,
                clipped_mesh.1.indices.len() as isize * 4,
                clipped_mesh.1.vertices.len() as isize * 5 * 4,
                clipped_mesh.1.vertices.as_ptr() as *const c_void,
            );

            gl::VertexArrayVertexBuffer
            (
                egui_state.vao,
                0,
                egui_state.vbo,
                clipped_mesh.1.indices.len() as isize * 4,
                5 * 4,
            );

            gl::DrawElements(gl::TRIANGLES, clipped_mesh.1.indices.len() as i32, gl::UNSIGNED_INT, ptr::null::<c_void>());
        }
    }
}

pub fn setup_vertex_arrays_egui() -> (u32, u32)
{
    let (mut vao, mut vbo) = (0, 0);

    unsafe
    {
        gl::CreateBuffers(1, &mut vbo);
        gl::CreateVertexArrays(1, &mut vao);

        gl::VertexArrayElementBuffer(vao, vbo);

        gl::EnableVertexArrayAttrib(vao, 0);
        gl::EnableVertexArrayAttrib(vao, 1);
        gl::EnableVertexArrayAttrib(vao, 2);

        gl::VertexArrayAttribFormat //vertex
        (
            vao,
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            0 * 4,
        );

        gl::VertexArrayAttribFormat //uv
        (
            vao,
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            2 * 4,
        );

        gl::VertexArrayAttribFormat //color
        (
            vao,
            2,
            4,
            gl::UNSIGNED_BYTE,
            gl::FALSE,
            4 * 4,
        );

        gl::VertexArrayAttribBinding(vao, 0, 0);
        gl::VertexArrayAttribBinding(vao, 1, 0);
        gl::VertexArrayAttribBinding(vao, 2, 0);
    }

    (vao, vbo)
}

pub fn setup_texture_egui() -> u32
{
    let mut tex_e = 0;

    unsafe
    {
        gl::CreateTextures(gl::TEXTURE_2D, 1, &mut tex_e);
        gl::TextureParameteri(tex_e, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TextureParameteri(tex_e, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    }

    tex_e
}

pub fn update_texture_egui(tex_e: u32, pix: Vec<(u8, u8, u8, u8)>, x: usize, y: usize, width: usize, height: usize)
{
    unsafe
    {
        gl::TextureSubImage2D
        (
            tex_e,
            0,
            x as i32,
            y as i32,
            width as i32,
            height as i32,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pix.as_ptr() as *const c_void,
        );
    }
}

pub fn upload_texture_egui(tex_e: u32, pix: Vec<(u8, u8, u8, u8)>, width: usize, height: usize)
{
    unsafe
    {
        gl::TextureStorage2D
        (
            tex_e,
            1,
            gl::RGBA8,
            width as i32,
            height as i32,
        );

        gl::TextureSubImage2D
        (
            tex_e,
            0,
            0,
            0,
            width as i32,
            height as i32,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pix.as_ptr() as *const c_void,
        );
    }
}

pub fn create_program(vertex_src: &str, fragment_src: &str) -> u32
{
    let vertex_handler = compile_shader(vertex_src, gl::VERTEX_SHADER);
    let fragment_handler = compile_shader(fragment_src, gl::FRAGMENT_SHADER);

    unsafe
    {
        let program_id = gl::CreateProgram();
        gl::AttachShader(program_id, vertex_handler);
        gl::AttachShader(program_id, fragment_handler);

        gl::LinkProgram(program_id);
        gl::UseProgram(program_id);

        gl::DeleteShader(vertex_handler);
        gl::DeleteShader(fragment_handler);

        program_id
    }
}

fn compile_shader(source: &str, shader_type: u32) -> u32
{
    unsafe
    {
        let shader_handler = gl::CreateShader(shader_type);
        let c_str = CString::new(source.as_bytes()).unwrap();
        gl::ShaderSource(shader_handler, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader_handler);

        let mut success = i32::from(gl::FALSE);

        gl::GetShaderiv(shader_handler, gl::COMPILE_STATUS, &mut success);
        if success != i32::from(gl::TRUE)
        {
            let mut len = 0;
            gl::GetShaderiv(shader_handler, gl::INFO_LOG_LENGTH, &mut len);

            let mut info_log = vec![0; len as usize];
            gl::GetShaderInfoLog(shader_handler, len, ptr::null_mut(), info_log.as_mut_ptr() as *mut i8);
            println!("Shader compilation failed: {}", from_utf8(&info_log).unwrap());
            std::process::exit(-1);
        }

        shader_handler
    }
}

pub fn update_textures(tex_set: egui::epaint::ahash::AHashMap<egui::TextureId, egui::epaint::ImageDelta>, tex_e: u32)
{
    for (id, image_delta) in &tex_set
    {
        let pixels: Vec<(u8, u8, u8, u8)> = match &image_delta.image
        {
            egui::ImageData::Color(image) =>
            {
                image.pixels.iter().map(|color| color.to_tuple()).collect()
            }

            egui::ImageData::Alpha(image) =>
            {
                let gamma = 1.0;
                image.srgba_pixels(gamma).map(|color| color.to_tuple()).collect()
            }
        };

        let width = image_delta.image.width();
        let height = image_delta.image.height();

        if let Some(pos) = image_delta.pos
        {
            update_texture_egui(tex_e, pixels, pos[0], pos[1], width, height)
        }
        else
        {
            upload_texture_egui(tex_e, pixels, width, height);
        }
    }
}

fn translate_virtual_key_code(key: VirtualKeyCode) -> Option<egui::Key>
{
    use VirtualKeyCode::*;
    use egui::Key;

    Some
    (
        match key
        {
            Down => Key::ArrowDown,
            Left => Key::ArrowLeft,
            Right => Key::ArrowRight,
            Up => Key::ArrowUp,

            Escape => Key::Escape,
            Tab => Key::Tab,
            Back => Key::Backspace,
            Return => Key::Enter,
            Space => Key::Space,

            Insert => Key::Insert,
            Delete => Key::Delete,
            Home => Key::Home,
            End => Key::End,
            PageUp => Key::PageUp,
            PageDown => Key::PageDown,

            Key0 | Numpad0 => Key::Num0,
            Key1 | Numpad1 => Key::Num1,
            Key2 | Numpad2 => Key::Num2,
            Key3 | Numpad3 => Key::Num3,
            Key4 | Numpad4 => Key::Num4,
            Key5 | Numpad5 => Key::Num5,
            Key6 | Numpad6 => Key::Num6,
            Key7 | Numpad7 => Key::Num7,
            Key8 | Numpad8 => Key::Num8,
            Key9 | Numpad9 => Key::Num9,

            A => Key::A,
            B => Key::B,
            C => Key::C,
            D => Key::D,
            E => Key::E,
            F => Key::F,
            G => Key::G,
            H => Key::H,
            I => Key::I,
            J => Key::J,
            K => Key::K,
            L => Key::L,
            M => Key::M,
            N => Key::N,
            O => Key::O,
            P => Key::P,
            Q => Key::Q,
            R => Key::R,
            S => Key::S,
            T => Key::T,
            U => Key::U,
            V => Key::V,
            W => Key::W,
            X => Key::X,
            Y => Key::Y,
            Z => Key::Z,

            _ => return None,
        }
    )
}

fn is_printable_char(chr: char) -> bool
{
    let is_in_private_use_area = ('\u{E000}' ..= '\u{F8FF}').contains(&chr)
        || ('\u{F0000}' ..= '\u{FFFFD}').contains(&chr)
        || ('\u{100000}' ..= '\u{10FFFD}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}

pub fn setup_egui_glutin(el: &glutin::event_loop::EventLoop<()>) -> EguiState
{
    let wb = glutin::window::WindowBuilder::new().with_inner_size(glutin::dpi::LogicalSize::new(1024, 768)).with_title("test");

    let windowed_context = glutin::ContextBuilder::new().build_windowed(wb, &el).unwrap();
    let windowed_context = unsafe{windowed_context.make_current().unwrap()};

    gl::load_with(|symbol| windowed_context.get_proc_address(symbol));
    unsafe
    {
        gl::Enable(gl::BLEND);
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::STENCIL_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::SCISSOR_TEST);
    }

    let (vao_e, vbo_e) = setup_vertex_arrays_egui();
    let vert_e = include_str!("shader_e.vert");
    let frag_e = include_str!("shader_e.frag");

    EguiState
    {
        windowed_context: windowed_context,

        ctx: egui::Context::default(),
        pos_in_points: None,
        raw_input: egui::RawInput::default(),

        vao: vao_e,
        vbo: vbo_e,
        tex: setup_texture_egui(),
        shader: create_program(vert_e, frag_e),

        buffer_size: 0,
    }
}

pub fn event_handling
(
    event: Event<()>,
    control_flow: &mut ControlFlow,
    windowed_context: &ContextWrapper<PossiblyCurrent, glutin::window::Window>,
    raw_input: &mut RawInput,
    pos_in_points: &mut Option<egui::Pos2>
)
{
    match event
    {
        Event::LoopDestroyed =>
        {
            //todo: write state file here?
        }

        Event::WindowEvent{event, ..} =>
        {
            match event
            {
                WindowEvent::ReceivedCharacter(ch) =>
                {
                    if is_printable_char(ch) && !raw_input.modifiers.ctrl
                    {
                        raw_input.events.push(egui::Event::Text(ch.to_string()));
                    }
                }

                WindowEvent::KeyboardInput{input, ..} =>
                {
                    if let Some(keycode) = input.virtual_keycode
                    {
                        let pressed = input.state == ElementState::Pressed;

                        if matches!(keycode, VirtualKeyCode::LAlt | VirtualKeyCode::RAlt)
                        {
                            raw_input.modifiers.alt = pressed;
                        }

                        if matches!(keycode, VirtualKeyCode::LControl | VirtualKeyCode::RControl)
                        {
                            raw_input.modifiers.ctrl = pressed;
                        }

                        if matches!(keycode, VirtualKeyCode::LShift | VirtualKeyCode::RShift)
                        {
                            raw_input.modifiers.shift = pressed;
                        }

                        if let Some(key) = translate_virtual_key_code(keycode)
                        {
                            if key == egui::Key::Escape && pressed
                            {
                                *control_flow = ControlFlow::Exit
                            }

                            raw_input.events.push
                            (
                                egui::Event::Key
                                {
                                    key,
                                    pressed,
                                    modifiers: raw_input.modifiers,
                                }
                            );
                        }
                    }
                }

                WindowEvent::CursorMoved{position, ..} =>
                {
                    let pos_in_points_temp = egui::pos2
                    (
                        position.x as f32 / 1.0,
                        position.y as f32 / 1.0,
                    );
                    *pos_in_points = Some(pos_in_points_temp);

                    raw_input.events.push(egui::Event::PointerMoved(pos_in_points_temp));
                }

                WindowEvent::MouseInput
                {
                    state,
                    button,
                    ..
                } =>
                {
                    if let Some(pos_in_points_temp) = pos_in_points
                    {
                        if let Some(button) =
                            match button
                            {
                                glutin::event::MouseButton::Left => Some(egui::PointerButton::Primary),
                                glutin::event::MouseButton::Right => Some(egui::PointerButton::Secondary),
                                glutin::event::MouseButton::Middle => Some(egui::PointerButton::Middle),
                                _ => None,
                            }
                        {
                            raw_input.events.push
                            (
                                egui::Event::PointerButton
                                {
                                    pos: *pos_in_points_temp,
                                    button,
                                    pressed: match state
                                    {
                                        glutin::event::ElementState::Pressed => true,
                                        glutin::event::ElementState::Released => false,
                                    },
                                    modifiers: Modifiers::default(),
                                }
                            );
                        }
                    }
                }

                WindowEvent::Resized(physical_size) =>
                {
                    windowed_context.resize(physical_size)
                }

                WindowEvent::CloseRequested =>
                {
                    *control_flow = ControlFlow::Exit
                }

                _ => ()
            }
        }

        _ => ()
    }
}
