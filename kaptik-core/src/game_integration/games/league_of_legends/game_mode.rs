// game_mode.rs

use shaco::model::ingame::GameMode;

pub fn game_mode_to_string(mode: &GameMode) -> String {
    match mode {
        GameMode::Classic => "5v5 Draft Pick".to_string(),
        GameMode::Arena => "Arena".to_string(),

        GameMode::Tutorial
        | GameMode::Tutorial1
        | GameMode::Tutorial2
        | GameMode::Tutorial3 => "Tutorial".to_string(),

        GameMode::Odin => "Dominion/Crystal Scar".to_string(),
        GameMode::Aram => "ARAM".to_string(),
        GameMode::Urf => "URF".to_string(),
        GameMode::PracticeTool => "Practice Tool".to_string(),
        GameMode::DoombotsTeemo => "Doombots".to_string(),
        GameMode::OneForAll => "One for All".to_string(),
        GameMode::Ascension => "Ascension".to_string(),
        GameMode::FirstBlood => "Snowdown Showdown".to_string(),
        GameMode::KingPoro => "Poroking".to_string(),
        GameMode::Siege => "Nexus Siege".to_string(),
        GameMode::Assassinate => "Blood Hunt Assassin".to_string(),
        GameMode::ARSR => "All Random Summoner's Rift".to_string(),
        GameMode::Darkstar => "Dark Star: Singularity".to_string(),
        GameMode::StarGuardian => "Star Guardian Invasion".to_string(),
        GameMode::Project => "PROJECT: Hunters".to_string(),
        GameMode::NexusBlitz => "Nexus Blitz".to_string(),
        GameMode::Odyssey => "Odyssey: Extraction".to_string(),
        GameMode::UltBook => "Ultimate Spellbook".to_string(),
        GameMode::Unknown => "".to_string(),
    }
}