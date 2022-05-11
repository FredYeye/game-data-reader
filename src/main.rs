#![windows_subsystem = "windows"]

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
mod game_data;

pub struct GuiState
{
    current_game: Option<game_data::GameData>,
    handle: HANDLE,
    offset: u64,

    update_timer: i8,
    timer_ticks: i8,

    graph: Graph,
    rank: u8,
}

#[derive(miniserde::Serialize, miniserde::Deserialize, Debug)]
struct Save
{
    //main window
    window_size: (u32, u32),
    // window_pos: (u32, u32),

    timer_ticks: i8,

    //rank graph
	rank_window_pos: (f32, f32),
	rank_window_width: f32,
	data_points: usize,
	color_r: (u8, u8),
    color_g: (u8, u8),
    color_b: (u8, u8),
	aspect: f32,
}

impl Default for Save
{
    fn default() -> Self
    {
        Self
        {
            window_size: (1024, 768),

            timer_ticks: 100,

            rank_window_pos: (20.0, 20.0),
            rank_window_width: 450.0,
            data_points: 240,
            color_r: (  0, 255),
            color_g: (255,   0),
            color_b: (  0,   0),
            aspect: 3.7,
        }
    }
}

struct Graph
{
    default_window_pos: (f32, f32),
    default_window_width: f32,
    values: std::collections::VecDeque<f32>,
    aspect: f32,
    color_start: [u8; 3],
    color_end: [u8; 3],
}

fn main()
{
    let save = if let Ok(str_ser) = std::fs::read_to_string("app.cfg")
    {
        miniserde::json::from_str(&str_ser)
        .expect("Unable to load app.cfg (was created in an older version most likely).")
    }
    else
    {
        Save::default()
    };

    let el = EventLoop::new();
    let mut egui_state = egui_glutin::setup_egui_glutin(&el, save.window_size);

    let mut last_time = std::time::Instant::now();
    let mut frame_time = std::time::Duration::new(0, 0);

    let mut gui_state = GuiState
    {
        current_game: None,
        handle: HANDLE::default(),
        offset: 0,

        update_timer: 0,
        timer_ticks: save.timer_ticks,

        graph: Graph
        {
            default_window_pos: save.rank_window_pos,
            default_window_width: save.rank_window_width,
            values:
            {
                let mut val = std::collections::VecDeque::new();
                val.resize(save.data_points, 0.0);
                val
            },
            aspect: save.aspect,
            color_start: [save.color_r.0, save.color_g.0, save.color_b.0],
            color_end: [save.color_r.1, save.color_g.1, save.color_b.1],
        },

        rank: 0,
    };

    el.run(move |event, _, control_flow|
    {
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(2));

        egui_glutin::event_handling(event, control_flow, &mut egui_state, &gui_state);

        let current_time = std::time::Instant::now();
        frame_time += current_time - last_time;
        last_time = current_time;

        let time = 20000;

        while frame_time >= std::time::Duration::from_micros(time)
        {
            frame_time -= std::time::Duration::from_micros(time);

            gui_state.update_timer -= 1;
            if gui_state.update_timer < 0
            {
                gui_state.update_timer = gui_state.timer_ticks;

                match gui_state.current_game
                {
                    Some(_) => update(&mut gui_state),
                    None => find_game(&mut gui_state),
                }
            }

            egui_state.ctx.begin_frame(egui_state.raw_input.take());

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
    let mut emu_info = None;

    let (pid_list, pid_count) = enum_processes();

    for x in 0 .. pid_count
    {
        unsafe
        {
            let handle_result = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, false, pid_list[x as usize]);
            if let Ok(handle) = handle_result
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
                            "bsnes.exe" => Some(game_data::Emulator::Bsnes),
                            "mame.exe" => Some(game_data::Emulator::Mame),
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
        let mut first_module = HINSTANCE::default();
        let mut lpcb_needed = 0;
        unsafe{ K32EnumProcessModules(handle, &mut first_module, std::mem::size_of::<HINSTANCE>() as u32, &mut lpcb_needed); }

        let mut info = MODULEINFO::default();
        unsafe{ K32GetModuleInformation(handle, first_module, &mut info, std::mem::size_of::<MODULEINFO>() as u32); }

        if emu == game_data::Emulator::Mame
        {
            if game_data::Emulator::get_mame_version(info.SizeOfImage) == 0
            {
                return; //unsupported mame version. kinda bootleg way to do this
            }
        }

        let game_name = get_game_name(&handle, &info, &emu);

        match game_name
        {
            Some(game) =>
            {
                gui_state.handle = handle;
                gui_state.current_game = Some(game.game_info());
                gui_state.offset = match emu
                {
                    game_data::Emulator::Bsnes => 0xB16D7C,
                    game_data::Emulator::Mame =>
                    {
                        let version = game_data::Emulator::get_mame_version(info.SizeOfImage);
                        let offset_list = game_data::Emulator::mame_game_offset(version, game);
                        get_mame_offset(&handle, info.lpBaseOfDll as u64, offset_list)
                    }
                };
            }

            None =>
            {
                unsafe{ CloseHandle(handle); }
            }
        };
    }
}

fn get_game_name(handle: &HANDLE, info: &MODULEINFO, emu: &game_data::Emulator) -> Option<game_data::Games>
{
    let game_name_offset = match emu
    {
        game_data::Emulator::Bsnes => 0xB151E8 as *const c_void,
        game_data::Emulator::Mame =>
        {
            let version = game_data::Emulator::get_mame_version(info.SizeOfImage);
            let name_offset = game_data::Emulator::get_mame_name_offset(version);

            (info.lpBaseOfDll as u64 + name_offset as u64) as *const c_void
        }
    };

    let mut raw_str = [0; 22];

    unsafe
    {
        let p_raw_str = raw_str.as_mut_ptr() as *mut _ as *mut c_void;
        let mut count = 0;
        ReadProcessMemory(handle, game_name_offset, p_raw_str, raw_str.len() - 1, &mut count);
    }

    let terminator = raw_str.into_iter().position(|x| x == 0).unwrap();

    match std::str::from_utf8(&raw_str[0 .. terminator])
    {
        Ok(name) => match emu
        {
            game_data::Emulator::Bsnes => game_data::Games::bsnes_game_name(name),
            game_data::Emulator::Mame => game_data::Games::mame_game_name(name),
        }

        Err(_) => None,
    }
}

fn enum_processes() -> ([u32; 384], u32)
{
    let mut pid_list = [0; 384];
    let mut pid_size = 0;
    unsafe{ K32EnumProcesses(pid_list.as_mut_ptr(), pid_list.len() as u32 * 4, &mut pid_size); }

    (pid_list, pid_size / 4)
}

fn get_mame_offset(handle: &HANDLE, dll_base: u64, offset_list: Vec<u64>) -> u64
{
    std::thread::sleep(std::time::Duration::from_secs(2)); //sleep because getting the offset while mame is loading the game can fail

    unsafe
    {
        let mut address = dll_base;

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

fn update(gui_state: &mut GuiState)
{
    //check if game window is closed. not perfect as user can load other game without closing the emulator
    //todo: check for string again probably
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
        println!("rank out of range: {}", gui_state.rank);
        gui_state.rank = 0;
    }

    gui_state.graph.values.pop_front();
    gui_state.graph.values.push_back(gui_state.rank as f32);
    gui_state.graph.values.make_contiguous();
}

fn create_ui(ctx: &mut Context, gui_state: &mut GuiState)
{
    let rect = egui::Rect
    {
        min: gui_state.graph.default_window_pos.into(),
        max: (gui_state.graph.default_window_width, 0.0).into(),
    };

    if let Some(data) = &gui_state.current_game
    {
        let response = egui::Window::new("Rank")
        .collapsible(false)
        .default_rect(rect)
        .show(ctx, |ui|
        {
            let plot = Plot::new("rank")
            .view_aspect(gui_state.graph.aspect)
            .allow_boxed_zoom(false)
            .allow_drag(false)
            // .y_grid_spacer(spacer) //figure out how this one works
            .show_axes([false, false]);

            plot
            .show(ui, |plot_ui|
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
                let len = gui_state.graph.values.len();
                gui_state.graph.values.clear();
                gui_state.graph.values.resize(len, 0.0);
            }

            ui.collapsing("Advanced", |ui|
            {
                let mut count = gui_state.graph.values.len();

                ui.add
                (
                    egui::DragValue::new(&mut count)
                    .speed(1.0)
                    .clamp_range(30 ..= 480)
                    .prefix("data points: ")
                );

                if count != gui_state.graph.values.len()
                {
                    gui_state.graph.values.resize(count, 0.0);
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

        let pos = &response.unwrap().response.rect;
        gui_state.graph.default_window_pos = (pos.min.x, pos.min.y);
        gui_state.graph.default_window_width = pos.max.x;
    }

    egui::Window::new("Game data reader").show(ctx, |ui|
    {
        ui.horizontal(|ui|
        {
            ui.add
            (
                egui::DragValue::new(&mut gui_state.timer_ticks)
                .speed(0.23)
                .clamp_range(5 ..= 125)
                .prefix("Ticks/update: ")
            );

            ui.label(format!("({:.2} updates/sec)", 50.0 / gui_state.timer_ticks as f32));
        });

        if gui_state.current_game.is_none()
        {
            ui.label("\nSearching for supported games...");
            ui.label("Once a game has been found, data will be shown automatically!");
        }
    });
}
