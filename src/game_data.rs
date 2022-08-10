use crate::update::{DataTypes, Rank, SmashTV};

#[derive(PartialEq)]
pub enum Emulator {
    Bsnes,
    Mame,
}

impl Emulator {
    pub fn name_offset(&self, size_of_image: u32, lp_base_of_dll: u64) -> u64 {
        match self {
            Emulator::Bsnes => 0xB151E8,
            Emulator::Mame => {
                let version = Self::get_mame_version(size_of_image);
                let name_offset = Self::get_mame_name_offset(version);

                lp_base_of_dll + name_offset as u64
            }
        }
    }

    pub fn get_mame_version(module_size: u32) -> u16 {
        match module_size {
            0x129FB000 => 242,
            0x12A82000 => 243,
            0x12C81000 => 246,
            _ => 0, //unsupported version
        }
    }

    pub fn get_mame_name_offset(version: u16) -> u32 {
        match version {
            242 => 0x11EC4450,
            243 => 0x11F3C970,
            246 => 0x1212E410,
            _ => todo!("unsupported mame version"),
        }
    }

    pub fn mame_game_offset(version: u16, games: Games) -> Vec<u64> {
        match version {
            242 => {
                match games {
                    Games::GhoulsArcade => vec![0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18],
                    Games::Gradius3Arcade => vec![0x11B72B48, 0x38, 0x150, 0x08, 0x10],
                    _ => todo!(),
                }
            }

            243 => {
                match games {
                    Games::GhoulsArcade => vec![0x11BF4390, 0x08, 0x10, 0x38, 0x40, 0x80, 0x18, 0x80, 0x18],
                    Games::Gradius3Arcade => vec![0x11BF4390, 0x28, 0x150, 0x08, 0x10],
                    _ => todo!(),
                }
            }

            246 => {
                match games {
                    Games::SpangArcade => vec![0x11DE8A68, 0x08, 0x10, 0x28, 0x70, 0xB8],
                    // 0x11DE8BD0 0x08 0x10 0x28 0x70 0xB8 0xD2
                    _ => todo!(),
                }
            }

            _ => todo!("unsupported mame version"),
        }
    }

    pub fn game_name(&self, name: &str) -> Option<Games> {
        match self {
            Emulator::Bsnes => {
                match name {
                    "gradius 3" | "GRADIUS 3" => Some(Games::Gradius3Snes),
                    "PARODIUS" => Some(Games::ParodiusSnes),
                    "SMASH T.V." => Some(Games::SmashTVSnes),
                    _ => None,
                }
            }

            Emulator::Mame => {
                match name {
                    "gradius3" | "gradius3a" | "gradius3j" | "gradius3js" => Some(Games::Gradius3Arcade),
                    "ghouls" | "ghoulsu" | "daimakai" | "daimakair" => Some(Games::GhoulsArcade),
                    "spang" | "spangj" | "sbbros" => Some(Games::SpangArcade),
                    _ => None,
                }
            }
        }
    }
}

pub struct GameData {
    pub id: Games,
    pub data_type: DataTypes,
}

pub enum Games {
    Gradius3Snes,
    ParodiusSnes,
    SmashTVSnes,

    GhoulsArcade,
    Gradius3Arcade,
    SpangArcade,
}

impl Games {
    pub fn format_rank(&self, rank: u8) -> u8 {
        match self {
            Games::GhoulsArcade => rank >> 3,
            _ => rank,
        }
    }

    pub fn game_info(&self) -> GameData {
        match self {
            Self::Gradius3Snes => GameData {
                id: Games::Gradius3Snes,
                data_type: DataTypes::Rank(
                    Rank {
                        data_points: std::collections::VecDeque::new(),
                        offset: 0x0084,
                        steps: 16,
                    }
                ),
            },

            Self::ParodiusSnes => GameData {
                id: Games::ParodiusSnes,
                data_type: DataTypes::Rank(
                    Rank {
                        data_points: std::collections::VecDeque::new(),
                        offset: 0x0088,
                        steps: 32,
                    }
                ),
            },

            Self::SmashTVSnes => GameData {
                id: Games::SmashTVSnes,
                data_type: DataTypes::SmashTV(
                    SmashTV {
                        enemy_type: [0; 7],
                        enemy_count: [0; 7],
                        spawn_timer: [0; 7],

                        active_enemies: [0; 1],
                    }
                ),
            },

            Self::GhoulsArcade => GameData {
                id: Games::GhoulsArcade,
                data_type: DataTypes::Rank(
                    Rank {
                        data_points: std::collections::VecDeque::new(),
                        offset: 0x092A,
                        steps: 16,
                    }
                ),
            },

            Self::Gradius3Arcade => GameData {
                id: Games::Gradius3Arcade,
                data_type: DataTypes::Rank(
                    Rank {
                        data_points: std::collections::VecDeque::new(),
                        offset: 0x39C0,
                        steps: 16,
                    }
                ),
            },

            Self::SpangArcade => GameData {
                id: Games::SpangArcade,
                data_type: DataTypes::Rank(
                    Rank {
                        data_points: std::collections::VecDeque::new(),
                        offset: 0xD2,
                        steps: 32,
                    }
                ),
            },
        }
    }
}
