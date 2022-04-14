use windows::{
    Win32::{
        Foundation::{HANDLE, HINSTANCE, CloseHandle, STILL_ACTIVE},
        System::Diagnostics::Debug::ReadProcessMemory,
        System::{
            Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION, GetExitCodeProcess},
            ProcessStatus::{K32EnumProcessModules, K32GetModuleInformation, MODULEINFO, K32EnumProcesses, K32GetModuleBaseNameA},
        },
    },
};

use std::ffi::c_void;
use egui::{Context, plot::{Plot, Line, Values, LineStyle}, Color32};
use glutin::event_loop::{ControlFlow, EventLoop};

mod egui_glutin;

struct GuiState
{
    current_game: Option<GameData>,
    handle: HANDLE,
    offset: u64,

    handle_timer: i8,
    memory_read_timer: i8,

    graph: Graph,
    rank: u8,
}

struct Graph
{
    value_count: u16,
    values: std::collections::VecDeque<f32>,
    aspect: f32,
    color_start: [u8; 3],
    color_end: [u8; 3],
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
    rank_offset: u16,
    rank_values: u8,
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
            "gradius 3" | "GRADIUS 3" => Some(Games::Gradius3Snes),
            "PARODIUS" => Some(Games::ParodiusSnes),
            _ => None,
        }
    }

    fn mame_game_name(name: &str) -> Option<Self>
    {
        match name
        {
            "gradius3" | "gradius3a" | "gradius3j" | "gradius3js" => Some(Games::Gradius3Arcade),
            "ghouls" | "ghoulsu" | "daimakai" | "daimakair" => Some(Games::GhoulsArcade),
            _ => None,
        }
    }

    fn mame_game_offset(&self) -> Vec<u64>
    {
        match self
        {
            Games::GhoulsArcade => vec![0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18],
            Games::Gradius3Arcade => vec![0x11B72B48, 0x38, 0x150, 0x8, 0x10],
            _ => unreachable!(),
        }
    }

    fn game_info(&self) -> GameData
    {
        match self
        {
            Self::Gradius3Snes => GameData
            {
                id: Games::Gradius3Snes,
                rank_offset: 0x0084,
                rank_values: 16,
            },

            Self::ParodiusSnes => GameData
            {
                id: Games::ParodiusSnes,
                rank_offset: 0x0088,
                rank_values: 32,
            },

            Self::GhoulsArcade => GameData
            {
                id: Games::GhoulsArcade,
                rank_offset: 0x092A,
                rank_values: 16,
            },

            Self::Gradius3Arcade => GameData
            {
                id: Games::Gradius3Arcade,
                rank_offset: 0x39C0,
                rank_values: 16,
            },
        }
    }
}

fn main()
{
    let el = EventLoop::new();
    let mut egui_state = egui_glutin::setup_egui_glutin(&el);

    let mut last_time = std::time::Instant::now();
    let mut frame_time = std::time::Duration::new(0, 0);

    let mut gui_state = GuiState
    {
        current_game: None,
        handle: HANDLE::default(),
        offset: 0,

        handle_timer: 0,
        memory_read_timer: 0,

        graph: Graph
        {
            value_count: 240,
            values: std::collections::VecDeque::from([0.0; 240]),
            aspect: 3.7,
            color_start: [0, 255, 0],
            color_end: [255, 0, 0],
        },

        rank: 0,
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
        let mut raw_str = [0; 22];
        let base = match emu
        {
            Emulator::Bsnes => 0xB151E8 as *const c_void,
            Emulator::Mame =>
            {
                let name_offset = vec![0x11B72B48];
                let offset = get_mame_offset(&handle, name_offset);
                (offset + 0xD8) as *const c_void
            }
        };

        unsafe
        {
            let p_raw_str = raw_str.as_mut_ptr() as *mut _ as *mut c_void;
            let mut count = 0;
            ReadProcessMemory(handle, base, p_raw_str, raw_str.len() - 1, &mut count);
        }

        let terminator = raw_str.into_iter().position(|x| x == 0).unwrap();

        let game_name = match std::str::from_utf8(&raw_str[0 .. terminator])
        {
            Ok(name) => match emu
            {
                Emulator::Bsnes => Games::bsnes_game_name(name),
                Emulator::Mame => Games::mame_game_name(name),
            }

            Err(e) => panic!("failed to get convert game name to string: {e}"),
        };

        match game_name
        {
            Some(game) =>
            {
                gui_state.handle = handle;
                gui_state.current_game = Some(game.game_info());
                gui_state.offset = match emu
                {
                    Emulator::Bsnes => 0xB16D7C,
                    Emulator::Mame => get_mame_offset(&handle, game.mame_game_offset()),
                };
            }

            None =>
            {
                unsafe{ CloseHandle(handle); }
            }
        };
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
        //todo: maybe log to some misc log window instead
        // println!("rank out of range: {}", gui_state.rank);
        gui_state.rank = 0;
    }

    gui_state.graph.values.pop_front();
    gui_state.graph.values.push_back(gui_state.rank as f32);
    gui_state.graph.values.make_contiguous();
}

fn create_ui(ctx: &mut Context, gui_state: &mut GuiState)
{
    if let Some(data) = &gui_state.current_game
    {
        egui::Window::new("Rank").show(ctx, |ui|
        {
            let plot = Plot::new("rank")
            .view_aspect(gui_state.graph.aspect)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .show_axes([false, false]);

            plot.show(ui, |plot_ui|
            {
                plot_ui.hline(egui::plot::HLine::new(0.0).color(Color32::DARK_GRAY));
                plot_ui.hline(egui::plot::HLine::new((data.rank_values - 1) as f32).color(Color32::DARK_GRAY));

                let mut rgb = [0; 3];

                for x in 0 .. 3
                {
                    let diff = gui_state.graph.color_end[x] as i16 - gui_state.graph.color_start[x] as i16;
                    
                    let step = match diff == 0
                    {
                        false => diff as f32 / (data.rank_values - 1) as f32,
                        true => 0.0,
                    };

                    rgb[x] = (gui_state.graph.color_start[x] as f32 + gui_state.rank as f32 * step).round() as u8;
                }

                plot_ui.line
                (
                    Line::new(Values::from_ys_f32(gui_state.graph.values.as_slices().0))
                    .color(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                    .style(LineStyle::Solid)
                )
            });

            if ui.button("Clear").clicked()
            {
                gui_state.graph.values = std::collections::VecDeque::new();
                gui_state.graph.values.resize(gui_state.graph.value_count as usize, 0.0);
            }

            ui.collapsing("Advanced", |ui|
            {
                let count = gui_state.graph.value_count;

                ui.add
                (
                    egui::DragValue::new(&mut gui_state.graph.value_count)
                    .speed(1.0)
                    .clamp_range(30 ..= 480)
                    .prefix("data points: ")
                );

                if count != gui_state.graph.value_count
                {
                    gui_state.graph.values.resize(gui_state.graph.value_count as usize, 0.0);
                }

                ui.add
                (
                    egui::DragValue::new(&mut gui_state.graph.aspect)
                    .speed(0.1)
                    .clamp_range(2.0 ..= 8.0)
                    .prefix("width/height ratio: ")
                );

                ui.horizontal(|ui|
                {
                    ui.label("Low/high rank colors: ");
                    ui.color_edit_button_srgb(&mut gui_state.graph.color_start);
                    ui.color_edit_button_srgb(&mut gui_state.graph.color_end);
                });
            });
        });
    }
    else
    {
        egui::Window::new("Game data reader").show(ctx, |ui|
        {
            ui.label("Searching for supported games...");
            ui.label("Once a game has been found, data will be shown automatically!");
        });
    }
}
