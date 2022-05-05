#[derive(PartialEq)]
pub enum Emulator
{
    Bsnes,
    Mame,
}

pub struct GameData
{
    pub id: Games,
    pub rank_offset: u16,
    pub rank_values: u8,
}

#[derive(PartialEq)]
pub enum Games
{
    Gradius3Snes,
    ParodiusSnes,
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

    pub fn mame_game_offset(&self) -> Vec<u64>
    {
        match self
        {
            Games::GhoulsArcade => vec![0x11B72B48, 0x08, 0x10, 0x28, 0x38, 0x60, 0x18, 0x80, 0x18],
            Games::Gradius3Arcade => vec![0x11B72B48, 0x38, 0x150, 0x8, 0x10],
            _ => unreachable!(),
        }
    }

    pub fn game_info(&self) -> GameData
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
