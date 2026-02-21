// maps.rs

/// Convert a queue ID to a human-readable game mode string.
pub fn queue_id_to_mode(queue_id: &str) -> &'static str {
    match queue_id {
        "competitive"        => "Competitive",
        "unrated"            => "Unrated",
        "spikerush"          => "Spike Rush",
        "deathmatch"         => "Deathmatch",
        "ggteam"             => "Escalation",
        "onefa"              => "Replication",
        "snowball"           => "Snowball Fight",
        "custom"             => "Custom Game",
        "hurm"               => "Team Deathmatch",
        "premier"            => "Premier",
        "swiftplay"          => "Swiftplay",
        _                    => "Valorant",
    }
}