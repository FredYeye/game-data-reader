#![windows_subsystem = "windows"]

use egui::{Context, plot::{Plot, Line, Values as EguiValues, LineStyle}, Color32, RichText};
use glutin::event_loop::{ControlFlow, EventLoop};
use update::CurrentGame;

mod egui_glutin;
mod game_data;
mod update;

pub struct GuiState {
    update_timer: i8,
    timer_ticks: i8,

    graph: Graph,
}

#[derive(miniserde::Serialize, miniserde::Deserialize, Debug)]
struct Save {
    //main window
    window_size: (u32, u32),
    // window_pos: (u32, u32),

    timer_ticks: i8,

    //rank graph
	rank_window_pos: (f32, f32),
	rank_window_width: f32,
	data_point_len: u16,
	color_r: (u8, u8),
    color_g: (u8, u8),
    color_b: (u8, u8),
	aspect: f32,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            window_size: (1024, 768),

            timer_ticks: 100,

            rank_window_pos: (20.0, 20.0),
            rank_window_width: 450.0,
            data_point_len: 240,
            color_r: (  0, 255),
            color_g: (255,   0),
            color_b: (  0,   0),
            aspect: 3.7,
        }
    }
}

struct Graph {
    default_window_pos: (f32, f32),
    default_window_width: f32,
    data_point_len: u16,
    aspect: f32,
    color_start: [u8; 3],
    color_end: [u8; 3],
}

fn main() {
    let save = if let Ok(str_ser) = std::fs::read_to_string("app.cfg") {
        miniserde::json::from_str(&str_ser)
        .expect("Unable to load app.cfg (was created in an older version most likely).")
    }
    else {
        Save::default()
    };

    let el = EventLoop::new();
    let mut egui_state = egui_glutin::setup_egui_glutin(&el, save.window_size);

    let mut last_time = std::time::Instant::now();
    let mut frame_time = std::time::Duration::new(0, 0);

    let mut gui_state = GuiState {
        update_timer: 0,
        timer_ticks: save.timer_ticks,

        graph: Graph {
            default_window_pos: save.rank_window_pos,
            default_window_width: save.rank_window_width,
            data_point_len: save.data_point_len,
            aspect: save.aspect,
            color_start: [save.color_r.0, save.color_g.0, save.color_b.0],
            color_end: [save.color_r.1, save.color_g.1, save.color_b.1],
        },
    };

    let mut current_game = None;

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(2));

        egui_glutin::event_handling(event, control_flow, &mut egui_state, &gui_state);

        let current_time = std::time::Instant::now();
        frame_time += current_time - last_time;
        last_time = current_time;

        let time = 20000;

        while frame_time >= std::time::Duration::from_micros(time) {
            frame_time -= std::time::Duration::from_micros(time);

            gui_state.update_timer -= 1;
            if gui_state.update_timer < 0 {
                gui_state.update_timer = gui_state.timer_ticks;

                match &mut current_game {
                    Some(current_game2) => {
                        if update::check_still_running(current_game2) {
                            update::update(current_game2);
                        }
                        else {
                            current_game = None;
                        }
                    }

                    None => current_game = update::find_game(),
                }

                //todo: kind of a hack. probably pass in guistate to find_game instead?
                if let Some(current_game2) = &mut current_game {
                    if let update::DataTypes::Rank(update::Rank{ data_points: data_points2, offset: _, steps: _}) = &mut current_game2.game.data_type {
                        data_points2.resize(gui_state.graph.data_point_len as usize, 0.0);
                    }
                }
            }

            egui_state.ctx.begin_frame(egui_state.raw_input.take());

            create_ui(&mut egui_state.ctx, &mut gui_state, &mut current_game); // add panels, windows and widgets to `egui_ctx` here

            let full_output = egui_state.ctx.end_frame();
            let clipped_meshes = egui_state.ctx.tessellate(full_output.shapes); // create triangles to paint
            // my_integration.set_cursor_icon(output.cursor_icon);
            egui_glutin::update_textures(full_output.textures_delta.set, egui_state.tex);
            egui_glutin::paint_egui(clipped_meshes, &mut egui_state);

            for &id in &full_output.textures_delta.free {
                todo!();
            }

            egui_state.windowed_context.swap_buffers().unwrap();
        }
    });
}

fn create_ui(ctx: &mut Context, gui_state: &mut GuiState, current_game: &mut Option<CurrentGame>) {
    if let Some(current_game2) = current_game {
        match &mut current_game2.game.data_type {
            update::DataTypes::Rank(rank) => rank_graph(ctx, gui_state, rank),
            update::DataTypes::SmashTV(smash_tv) => smash_tv_display(ctx, gui_state, smash_tv),
        }
    }

    egui::Window::new("Game data reader").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add (
                egui::DragValue::new(&mut gui_state.timer_ticks)
                .speed(0.23)
                .clamp_range(5 ..= 125)
                .prefix("Ticks/update: ")
            );

            ui.label(format!("({:.2} updates/sec)", 50.0 / gui_state.timer_ticks as f32));
        });

        if current_game.is_none() {
            ui.label("\nSearching for supported games...");
            ui.label("Once a game has been found, data will be shown automatically!");
        }
    });
}

fn rank_graph(ctx: &mut Context, gui_state: &mut GuiState, rank: &mut update::Rank) {
    let rect = egui::Rect {
        min: gui_state.graph.default_window_pos.into(),
        max: (gui_state.graph.default_window_width, 0.0).into(),
    };

    let response = egui::Window::new("Rank")
    .collapsible(false)
    .default_rect(rect)
    .show(ctx, |ui| {
        let plot = Plot::new("rank")
        .view_aspect(gui_state.graph.aspect)
        .allow_boxed_zoom(false)
        .allow_drag(false)
        // .y_grid_spacer(spacer) //figure out how this one works
        .show_axes([false, false]);

        plot
        .show(ui, |plot_ui| {
            plot_ui.hline(egui::plot::HLine::new(0.0).color(Color32::DARK_GRAY));
            plot_ui.hline(egui::plot::HLine::new((rank.steps - 1) as f32).color(Color32::DARK_GRAY));

            let mut rgb = [0; 3];

            for x in 0 .. 3 {
                let diff = gui_state.graph.color_end[x] as f32 - gui_state.graph.color_start[x] as f32;
                
                let step = match diff == 0.0 {
                    false => diff / (rank.steps - 1) as f32,
                    true => 0.0,
                };

                rgb[x] = (gui_state.graph.color_start[x] as f32 + rank.data_points.back().unwrap() * step).round() as u8;
            }

            plot_ui.line(
                Line::new(EguiValues::from_ys_f32(rank.data_points.as_slices().0))
                .color(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                .style(LineStyle::Solid)
            )
        });

        if ui.button("Clear").clicked() {
            let len = rank.data_points.len();
            rank.data_points.clear();
            rank.data_points.resize(len, 0.0);
        }

        ui.collapsing("Advanced", |ui| {
            let mut count = rank.data_points.len();

            ui.add(
                egui::DragValue::new(&mut count)
                .speed(0.9)
                .clamp_range(30 ..= 500)
                .prefix("data points: ")
            );

            if count != rank.data_points.len() {
                rank.data_points.resize(count, 0.0);
            }

            ui.add(
                egui::DragValue::new(&mut gui_state.graph.aspect)
                .speed(0.1)
                .clamp_range(2.0 ..= 8.0)
                .prefix("width/height ratio: ")
            );

            ui.horizontal(|ui| {
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

fn smash_tv_display(ctx: &mut Context, gui_state: &mut GuiState, smash_tv: &update::SmashTV) {
    egui::Window::new("Smash TV")
    .collapsible(false)
    .show(ctx, |ui| {
        ui.label(
            RichText::new("Wave         | Count | Spawn timer\n----------------------------")
            .monospace()
        );

        for x in 0 .. 7 {
            if smash_tv.enemy_type[x] != 0 {
                let names = [
                    "Empty", "Grunt", "Wall gunner", "Worm",
                    "Red flier", "Snakes", "Snake man", "Laser orb",
                    "Tank", "Red cluster", "Mr. Shrapnel", "Worm (blue)",
                    "Electric orb", "?", "?", "Mine",
                ];

                ui.label(
                    RichText::new(
                        format!(
                            "{:12} | {:>5} | {:.1} | {}",
                            names[smash_tv.enemy_type[x] as usize & 0x0F],
                            smash_tv.enemy_count[x],
                            smash_tv.spawn_timer[x] as f32 / 60.0,
                            smash_tv.active_enemies[0] as u16 + smash_tv.enemy_count[x],
                        )
                    )
                    .monospace()
                );
            }
        }
    });
}
