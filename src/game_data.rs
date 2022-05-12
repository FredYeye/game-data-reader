#[derive(PartialEq)]
pub enum Emulator
{
    Bsnes,
    Mame,
}

impl Emulator
{
    pub fn get_mame_version(module_size: u32) -> u16
    {
        match module_size
        {
            0x129FB000 => 242,
            0x12A82000 => 243,
            _ => 0, //unsupported version
        }
    }

    pub fn get_mame_name_offset(version: u16) -> u32
    {
        match version
        {
            242 => 0x11EC4450,
            243 => 0x11F3C970,
            _ => todo!("unsupported mame version"),
        }
    }

    pub fn mame_game_offset(version: u16, games: Games) -> Vec<u64>
    {
        match version
        {
            242 =>
            {
                match games
                {
                    Games::GhoulsArcade => vec![0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18],
                    Games::Gradius3Arcade => vec![0x11B72B48, 0x38, 0x150, 0x8, 0x10],
                    _ => unreachable!(),
                }
            }

            243 =>
            {
                match games
                {
                    Games::GhoulsArcade => vec![0x11BF4390, 0x8, 0x10, 0x38, 0x40, 0x80, 0x18, 0x80, 0x18],
                    Games::Gradius3Arcade => vec![0x11BF4390, 0x28, 0x150, 0x8, 0x10],
                    _ => unreachable!(),
                }
            }

            _ => todo!("unsupported mame version"),
        }
    }
}

pub enum DataTypes
{
    Rank{offset: u16, values: u8},
    SmashTV,
}

pub struct GameData
{
    pub id: Games,
    pub data_type: DataTypes,
}

#[derive(PartialEq)]
pub enum Games
{
    Gradius3Snes,
    ParodiusSnes,
    SmashTVSnes,

    GhoulsArcade,
    Gradius3Arcade,
}

impl Games
{
    pub fn format_rank(&self, rank: u8) -> u8
    {
        match self
        {
            Games::GhoulsArcade => rank >> 3,
            _ => rank,
        }
    }

    pub fn bsnes_game_name(name: &str) -> Option<Self>
    {
        match name
        {
            "gradius 3" | "GRADIUS 3" => Some(Games::Gradius3Snes),
            "PARODIUS" => Some(Games::ParodiusSnes),
            "SMASH T.V." => Some(Games::SmashTVSnes),
            _ => None,
        }
    }

    pub fn mame_game_name(name: &str) -> Option<Self>
    {
        match name
        {
            "gradius3" | "gradius3a" | "gradius3j" | "gradius3js" => Some(Games::Gradius3Arcade),
            "ghouls" | "ghoulsu" | "daimakai" | "daimakair" => Some(Games::GhoulsArcade),
            _ => None,
        }
    }

    pub fn game_info(&self) -> GameData
    {
        match self
        {
            Self::Gradius3Snes => GameData
            {
                id: Games::Gradius3Snes,
                data_type: DataTypes::Rank{offset: 0x0084, values: 16},
            },

            Self::ParodiusSnes => GameData
            {
                id: Games::ParodiusSnes,
                data_type: DataTypes::Rank{offset: 0x0088, values: 32},
            },

            Self::SmashTVSnes => GameData
            {
                id: Games::SmashTVSnes,
                data_type: DataTypes::SmashTV,
            },

            Self::GhoulsArcade => GameData
            {
                id: Games::GhoulsArcade,
                data_type: DataTypes::Rank{offset: 0x092A, values: 16},
            },

            Self::Gradius3Arcade => GameData
            {
                id: Games::Gradius3Arcade,
                data_type: DataTypes::Rank{offset: 0x39C0, values: 16},
            },
        }
    }
}
