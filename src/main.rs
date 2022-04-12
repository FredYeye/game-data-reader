use windows::{
    Win32::{
        Foundation::{HANDLE, HINSTANCE},
        UI::WindowsAndMessaging::{FindWindowA, GetWindowThreadProcessId},
        System::Diagnostics::Debug::ReadProcessMemory,
        System::{
            Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION},
            ProcessStatus::{K32EnumProcessModules, K32GetModuleInformation, MODULEINFO},
        },
    },
    core::PCSTR,
};

use std::ffi::c_void;
use egui::{Context, plot::{Plot, Line, Values, LineStyle}, Color32, RawInput};
use glutin::{ContextWrapper, PossiblyCurrent};
use glutin::event_loop::{ControlFlow, EventLoop};

mod egui_glutin;

pub struct EguiState
{
    windowed_context: ContextWrapper<PossiblyCurrent, glutin::window::Window>,

    ctx: Context,
    pos_in_points: Option<egui::Pos2>,
    raw_input: RawInput,

    vao: u32,
    vbo: u32,
    tex: u32,
    shader: u32,

    buffer_size: u32,
}

struct GuiState
{
    p_handle: HANDLE,
    offset: u64,
    handle_timer: i8,
    memory_read_timer: i8,

    graph: std::collections::VecDeque<f32>,
    rank: u8,
    current_game: Option<GameData>,
}

#[derive(PartialEq)]
enum Games
{
    Gradius3Snes, ParodiusSnes,
    GhoulsArcade,
}

impl Games
{
    fn format_rank(&self, rank: u8) -> u8
    {
        match self
        {
            Games::Gradius3Snes => rank,
            Games::ParodiusSnes => rank,
            Games::GhoulsArcade => rank >> 3,
        }
    }
}

#[derive(PartialEq)]
enum Emulator
{
    Mame,
    Bsnes,
}

struct GameData
{
    id: Games,
    name: String,
    emulator: Emulator,
    rank_offset: u16,
    rank_values: u8,
}

fn main()
{
    let el = EventLoop::new();
    let mut egui_state = egui_glutin::setup_egui_glutin(&el);

    let mut last_time = std::time::Instant::now();
    let mut frame_time = std::time::Duration::new(0, 0);

    let mut gui_state = GuiState
    {
        p_handle: HANDLE::default(),
        offset: 0,
        handle_timer: 0,
        memory_read_timer: 0,

        graph: std::collections::VecDeque::from([0.0; 240]),
        rank: 0,
        current_game: None,
    };

    el.run(move |event, _, control_flow|
    {
        *control_flow = ControlFlow::Poll;

        egui_glutin::event_handling(event, control_flow, &egui_state.windowed_context, &mut egui_state.raw_input, &mut egui_state.pos_in_points);

        let current_time = std::time::Instant::now();
        frame_time += current_time - last_time;
        last_time = current_time;

        while frame_time >= std::time::Duration::from_micros(33333)
        {
            frame_time -= std::time::Duration::from_micros(33333);

            egui_state.ctx.begin_frame(egui_state.raw_input.take());
            find_game(&mut gui_state);
            update(&mut gui_state);
            create_ui(&mut egui_state.ctx, &mut gui_state); // add panels, windows and widgets to `egui_ctx` here
            let full_output = egui_state.ctx.end_frame();
            let clipped_meshes = egui_state.ctx.tessellate(full_output.shapes); // create triangles to paint
            // my_integration.set_cursor_icon(output.cursor_icon);
            egui_glutin::update_textures(full_output.textures_delta.set, egui_state.tex);

            egui_glutin::paint_egui(clipped_meshes, &mut egui_state);

            for &id in &full_output.textures_delta.free
            {
                todo!();
            }

            egui_state.windowed_context.swap_buffers().unwrap();
        }
    });
}

fn valid_games() -> Vec<GameData>
{
    vec!
    [
        GameData
        {
            id: Games::Gradius3Snes,
            name: String::from("Gradius III"),
            emulator: Emulator::Bsnes,
            rank_offset: 0x0084,
            rank_values: 16,
        },

        GameData
        {
            id: Games::ParodiusSnes,
            name: String::from("Parodius Da! - Shinwa kara Owarai e (japan)"),
            emulator: Emulator::Bsnes,
            rank_offset: 0x0088,
            rank_values: 32,
        },

        GameData
        {
            id: Games::GhoulsArcade,
            name: String::from("Daimakaimura (Japan) [daimakai] - MAME 0.242 (LLP64)"),
            emulator: Emulator::Mame,
            rank_offset: 0x092A,
            rank_values: 16,
        },
    ]
}

fn find_game(gui_state: &mut GuiState)
{
    gui_state.handle_timer -= 1;

    if gui_state.handle_timer < 0
    {
        gui_state.handle_timer = 30;

        if let Some(game) = &gui_state.current_game //already found a game window
        {
            unsafe
            {
                if FindWindowA(PCSTR::default(), game.name.as_str()).ok().is_err()
                {
                    gui_state.current_game = None;
                    gui_state.p_handle.0 = 0;
                }
            }
        }
        else //search for a game window
        {
            for game in valid_games()
            {
                unsafe
                {
                    if let Ok(hwnd2) = FindWindowA(PCSTR::default(), game.name.as_str()).ok()
                    {
                        let mut process_id = 0;
                        GetWindowThreadProcessId(hwnd2, &mut process_id);
                        gui_state.p_handle = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, process_id);

                        gui_state.offset = match game.emulator
                        {
                            Emulator::Mame =>get_mame_offset(&gui_state.p_handle),
                            Emulator::Bsnes => 0xB16D7C,
                        };

                        gui_state.current_game = Some(game);
                        break;
                    }
                }
            }
        }
    }
}

fn get_mame_offset(handle: &HANDLE) -> u64
{
    //sleep because getting the offset while mame is loading the game can fail
    std::thread::sleep(std::time::Duration::from_secs(1));

    unsafe
    {
        let mut first_module = HINSTANCE::default();
        let mut lpcb_needed = 0;
        K32EnumProcessModules(handle, &mut first_module, std::mem::size_of::<HINSTANCE>() as u32, &mut lpcb_needed);

        let mut info = MODULEINFO::default();
        K32GetModuleInformation(handle, first_module, &mut info, std::mem::size_of::<MODULEINFO>() as u32);

        let mut address = info.lpBaseOfDll as u64;

        let offsets = [0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18];

        for offset in offsets
        {
            let base = (address + offset) as *const c_void;
            let p_address = &mut address as *mut _ as *mut c_void;
            let mut count = 0;
            ReadProcessMemory(handle, base, p_address, 8, &mut count);
        }

        address
    }
}

fn update(gui_state: &mut GuiState)
{
    if let Some(game) = &gui_state.current_game
    {
        gui_state.memory_read_timer -= 1;

        if gui_state.memory_read_timer < 0
        {
            gui_state.memory_read_timer = 60; //30 = 1s

            let mut rank = 0;

            unsafe
            {
                let base = (gui_state.offset + game.rank_offset as u64) as *const c_void;
                let p_rank = &mut rank as *mut _ as *mut c_void;
                let mut count = 0;
                ReadProcessMemory(gui_state.p_handle, base, p_rank, 1, &mut count);
            }

            gui_state.rank = game.id.format_rank(rank);

            if gui_state.rank >= game.rank_values
            {
                println!("rank out of range: {}", gui_state.rank);
                gui_state.rank = 0;
            }

            gui_state.graph.pop_front();
            gui_state.graph.push_back(gui_state.rank as f32);
            gui_state.graph.make_contiguous();
        }
    }
}

fn create_ui(ctx: &mut Context, gui_state: &mut GuiState)
{
    if let Some(data) = &gui_state.current_game
    {
        egui::Window::new("Rank").show(ctx, |ui|
        {
            ui.set_min_height(150.0);

            let plot = Plot::new("rank")
            .view_aspect(3.7)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .show_axes([false, false]);

            plot.show(ui, |plot_ui|
            {
                plot_ui.hline(egui::plot::HLine::new(0.0).color(Color32::DARK_GRAY));
                plot_ui.hline(egui::plot::HLine::new((data.rank_values - 1) as f32).color(Color32::DARK_GRAY));

                let red = ((gui_state.rank as f32 / (data.rank_values - 1) as f32) * 255.0) as u8;
                let green = 255 - red;

                plot_ui.line
                (
                    Line::new(Values::from_ys_f32(gui_state.graph.as_slices().0))
                    .color(Color32::from_rgb(red, green, 0))
                    .style(LineStyle::Solid)
                )
            });

            if ui.button("Clear").clicked()
            {
                gui_state.graph = std::collections::VecDeque::from([0.0; 240]);
            }
        });
    }
}
