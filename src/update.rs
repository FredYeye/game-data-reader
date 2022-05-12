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

use crate::{GuiState, game_data};
use std::ffi::c_void;

pub struct CurrentGame
{
    pub game: game_data::GameData,
    handle: HANDLE,
    offset: u64,
}

pub fn find_game(gui_state: &mut GuiState) -> Option<CurrentGame>
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
                return None; //unsupported mame version. kinda bootleg way to do this
            }
        }

        let game_name = get_game_name(&handle, &info, &emu);

        match game_name
        {
            Some(game) =>
            {
                Some(CurrentGame
                {
                    game: game.game_info(),
                    handle: handle,
                    offset: match emu
                    {
                        game_data::Emulator::Bsnes => 0xB16D7C,
                        game_data::Emulator::Mame =>
                        {
                            let version = game_data::Emulator::get_mame_version(info.SizeOfImage);
                            let offset_list = game_data::Emulator::mame_game_offset(version, game);
                            get_mame_offset(&handle, info.lpBaseOfDll as u64, offset_list)
                        }
                    },
                })
            }

            None =>
            {
                unsafe{ CloseHandle(handle); }
                None
            }
        }
    }
    else
    {
        None
    }
}

fn enum_processes() -> ([u32; 384], u32)
{
    let mut pid_list = [0; 384];
    let mut pid_size = 0;
    unsafe{ K32EnumProcesses(pid_list.as_mut_ptr(), pid_list.len() as u32 * 4, &mut pid_size); }

    (pid_list, pid_size / 4)
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

pub fn check_still_running(current_game: &CurrentGame) -> bool
{
    //check if game window is closed. not perfect as user can load other game without closing the emulator
    //todo: check for string again probably
    let mut exit_code = 0;
    unsafe{ GetExitCodeProcess(current_game.handle, &mut exit_code); }
    
    exit_code == STILL_ACTIVE.0 as u32
}

pub fn update(gui_state: &mut GuiState, current_game: &CurrentGame)
{
    match current_game.game.id
    {
        game_data::Games::Gradius3Snes => update_rank(gui_state, current_game),
        game_data::Games::ParodiusSnes => update_rank(gui_state, current_game),
        game_data::Games::SmashTVSnes => todo!(),
        game_data::Games::GhoulsArcade => update_rank(gui_state, current_game),
        game_data::Games::Gradius3Arcade => update_rank(gui_state, current_game),
    }
}

fn update_rank(gui_state: &mut GuiState, current_game: &CurrentGame)
{
    match current_game.game.data_type
    {
        game_data::DataTypes::Rank{offset, values} =>
        {
            let mut rank = 0;

            unsafe
            {
                let base = (current_game.offset + offset as u64) as *const c_void;
                let p_rank = &mut rank as *mut _ as *mut c_void;
                let mut count = 0;
                ReadProcessMemory(current_game.handle, base, p_rank, 1, &mut count);
            }
        
            gui_state.rank = current_game.game.id.format_rank(rank);
        
            if gui_state.rank >= values
            {
                println!("rank out of range: {}", gui_state.rank); //todo: maybe log to some misc log window instead
                gui_state.rank = 0;
            }
        
            gui_state.graph.values.pop_front();
            gui_state.graph.values.push_back(gui_state.rank as f32);
            gui_state.graph.values.make_contiguous();
        }
        _ => unsafe{ std::hint::unreachable_unchecked() },
    }
}
