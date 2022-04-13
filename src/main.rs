use windows::{
    Win32::{
        Foundation::{HANDLE, HINSTANCE, CloseHandle, STILL_ACTIVE},
        UI::WindowsAndMessaging::{FindWindowA, GetWindowThreadProcessId},
        System::Diagnostics::Debug::ReadProcessMemory,
        System::{
            Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION, GetExitCodeProcess},
            ProcessStatus::{K32EnumProcessModules, K32GetModuleInformation, MODULEINFO, K32EnumProcesses, K32GetModuleBaseNameA},
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
    handle: HANDLE,
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
    Gradius3Snes,
    ParodiusSnes,
    GhoulsArcade,
    Gradius3Arcade,
}

impl Games
{
    fn format_rank(&self, rank: u8) -> u8
    {
        match self
        {
            Games::GhoulsArcade => rank >> 3,
            _ => rank,
        }
    }

    fn bsnes_game_name(name: &str) -> Option<Self>
    {
        match name
        {
            "gradius 3" => Some(Games::Gradius3Snes),
            "PARODIUS" => Some(Games::ParodiusSnes),
            _ => None,
        }
    }

    fn game_info(&self) -> GameData
    {
        match self
        {
            Self::Gradius3Snes => GameData
            {
                id: Games::Gradius3Snes,
                name: String::from("Gradius III"), //unused
                // emulator: Emulator::Bsnes,
                rank_offset: 0x0084,
                rank_values: 16,
            },

            Self::ParodiusSnes => GameData
            {
                id: Games::ParodiusSnes,
                name: String::from("Parodius Da! - Shinwa kara Owarai e (japan)"), //unused
                // emulator: Emulator::Bsnes,
                rank_offset: 0x0088,
                rank_values: 32,
            },

            Self::GhoulsArcade => GameData
            {
                id: Games::GhoulsArcade,
                name: String::from("Daimakaimura (Japan) [daimakai] - MAME 0.242 (LLP64)"),
                // emulator: Emulator::Mame,
                rank_offset: 0x092A,
                rank_values: 16,
            },

            Self::Gradius3Arcade => GameData
            {
                id: Games::Gradius3Arcade,
                name: String::from("Gradius III (World, program code R) [gradius3] - MAME 0.242 (LLP64)"),
                // emulator: Emulator::Mame,
                rank_offset: 0x39C0,
                rank_values: 16,
            },
        }
    }
}

#[derive(PartialEq)]
enum Emulator
{
    Bsnes,
    Mame,
}

struct GameData
{
    id: Games,
    name: String,
    // emulator: Emulator,
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
        handle: HANDLE::default(),
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

            match gui_state.current_game
            {
                Some(_) => update(&mut gui_state),
                None => find_game(&mut gui_state),
            }

            // find_game(&mut gui_state);
            // update(&mut gui_state);
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

fn find_game(gui_state: &mut GuiState)
{
    if gui_state.handle_timer > 0
    {
        gui_state.handle_timer -= 1;
        return;
    }
    gui_state.handle_timer = 30;

    let mut emu_info = None;

    let (pid_list, pid_size) = enum_processes();

    for x in 0 .. pid_size / 4
    {
        unsafe
        {
            let handle = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, pid_list[x as usize]);
            if handle.ok().is_ok()
            {
                let mut first_module = HINSTANCE::default();
                let mut lpcb_needed = 0;
                K32EnumProcessModules(handle, &mut first_module, std::mem::size_of::<HINSTANCE>() as u32, &mut lpcb_needed);

                let mut module_name = [0; 256];
                let len = K32GetModuleBaseNameA(handle, first_module, &mut module_name);

                let emu = match std::str::from_utf8(&module_name[0 .. len as usize])
                {
                    Ok(str2) =>
                    {
                        match str2
                        {
                            "bsnes.exe" => Some(Emulator::Bsnes),
                            "mame.exe" => Some(Emulator::Mame),
                            _ => None,
                        }
                    }

                    Err(e) => panic!("failed to get convert module name to string: {e}"),
                };

                if let Some(emu2) = emu
                {
                    emu_info = Some((emu2, handle));
                }
                else
                {
                    CloseHandle(handle);
                }
            }
        }
    }

    if let Some((emu, handle)) = emu_info
    {
        match emu
        {
            Emulator::Bsnes =>
            {
                let mut raw_str = [0; 22];

                unsafe
                {
                    let base = 0xB151E8 as *const c_void;
                    let p_raw_str = raw_str.as_mut_ptr() as *mut _ as *mut c_void;
                    let mut count = 0;
                    ReadProcessMemory(handle, base, p_raw_str, 21, &mut count);
                }

                let terminator = raw_str.into_iter().position(|x| x == 0).unwrap();

                let game_name = match std::str::from_utf8(&raw_str[0 .. terminator])
                {
                    Ok(name) => Games::bsnes_game_name(name),
                    Err(e) => panic!("failed to get convert game name to string: {e}"),
                };

                match game_name
                {
                    Some(game) =>
                    {
                        gui_state.handle = handle;
                        gui_state.offset = 0xB16D7C;
                        gui_state.current_game = Some(game.game_info());
                    }

                    None =>
                    {
                        unsafe{ CloseHandle(handle); }
                    }
                };
            }

            Emulator::Mame =>
            {
                //todo: very bootleg! fix

                let ghouls_data =
                [
                    (Games::GhoulsArcade.game_info(), vec![0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18]),
                    (Games::Gradius3Arcade.game_info(), vec![0x11B72B48, 0x38, 0x150, 0x8, 0x10]),
                ];

                for games in ghouls_data
                {
                    unsafe
                    {
                        if let Ok(hwnd2) = FindWindowA(PCSTR::default(), games.0.name.as_str()).ok()
                        {
                            let mut process_id = 0;
                            GetWindowThreadProcessId(hwnd2, &mut process_id);
    
                            gui_state.handle = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, process_id);
                            gui_state.offset = get_mame_offset(&gui_state.handle, games.1);
                            gui_state.current_game = Some(games.0);
                        }
                    }
                }
            }
        }
    }
}

fn get_mame_offset(handle: &HANDLE, offset_list: Vec<u64>) -> u64
{
    //sleep because getting the offset while mame is loading the game can fail
    std::thread::sleep(std::time::Duration::from_secs(2));

    unsafe
    {
        let mut first_module = HINSTANCE::default();
        let mut lpcb_needed = 0;
        K32EnumProcessModules(handle, &mut first_module, std::mem::size_of::<HINSTANCE>() as u32, &mut lpcb_needed);

        let mut info = MODULEINFO::default();
        K32GetModuleInformation(handle, first_module, &mut info, std::mem::size_of::<MODULEINFO>() as u32);

        let mut address = info.lpBaseOfDll as u64;

        for offset in offset_list
        {
            let base = (address + offset) as *const c_void;
            let p_address = &mut address as *mut _ as *mut c_void;
            let mut count = 0;
            ReadProcessMemory(handle, base, p_address, 8, &mut count);
        }

        address
    }
}

fn enum_processes() -> ([u32; 384], u32)
{
    let mut pid_list = [0; 384];
    let mut pid_size = 0;
    unsafe{ K32EnumProcesses(pid_list.as_mut_ptr(), pid_list.len() as u32 * 4, &mut pid_size); }

    (pid_list, pid_size)
}

fn update(gui_state: &mut GuiState)
{
    if gui_state.memory_read_timer > 0
    {
        gui_state.memory_read_timer -= 1;
        return;
    }
    gui_state.memory_read_timer = 60; //30 = 1s

    //check if game window is closed. not perfect as user can load other game without closing the emulator
    let mut exit_code = 0;
    unsafe{ GetExitCodeProcess(gui_state.handle, &mut exit_code); }
    if exit_code != STILL_ACTIVE.0 as u32
    {
        gui_state.current_game = None;
        gui_state.handle.0 = 0;
        return;
    }

    let game = gui_state.current_game.as_ref().unwrap();

    let mut rank = 0;

    unsafe
    {
        let base = (gui_state.offset + game.rank_offset as u64) as *const c_void;
        let p_rank = &mut rank as *mut _ as *mut c_void;
        let mut count = 0;
        ReadProcessMemory(gui_state.handle, base, p_rank, 1, &mut count);
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

                let red = ((gui_state.rank as f32 / (data.rank_values - 1) as f32) * 255.0).round() as u8;
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
