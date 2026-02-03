/// Extrahiert den Spielnamen aus dem Fenstertitel
pub fn extract_game_name(window_title: &str) -> String {
    window_title
        .split(&['-', '(', ')', '™', '®'][..])
        .next()
        .unwrap_or(window_title)
        .trim()
        .replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_game_name() {
        assert_eq!(extract_game_name("Minecraft - Singleplayer"), "Minecraft");
        assert_eq!(extract_game_name("Counter-Strike 2"), "Counter");
        assert_eq!(extract_game_name("Elden Ring™"), "Elden_Ring");
        assert_eq!(extract_game_name("Game (Paused)"), "Game");
    }
}