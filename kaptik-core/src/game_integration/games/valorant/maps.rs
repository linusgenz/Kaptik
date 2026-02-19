// maps.rs

// maps.rs – Static lookups for Valorant map paths → human names,
// agent UUIDs → agent names, and queue IDs → game mode strings.

/// Convert a Valorant map asset path to a human-readable name.
///
/// Paths look like `"/Game/Maps/Ascent/Ascent"`. Returns the raw path
/// tail if no match is found.
pub fn map_id_to_name(map_id: &str) -> &'static str {
    match map_id {
        s if s.contains("Ascent")    => "Ascent",
        s if s.contains("Duality")   => "Bind",
        s if s.contains("Triad")     => "Haven",
        s if s.contains("Port")      => "Icebox",
        s if s.contains("Bonsai")    => "Split",
        s if s.contains("Foxtrot")   => "Breeze",
        s if s.contains("Canyon")    => "Fracture",
        s if s.contains("Pitt")      => "Pearl",
        s if s.contains("Jam")       => "Lotus",
        s if s.contains("Juliett")   => "Sunset",
        s if s.contains("Hurm")      => "Abyss",
        s if s.contains("VOID")      => "The Range",
        _ => "Unknown Map",
    }
}

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

/// Convert an agent UUID to a display name.
///
/// TODO add lookup here
pub fn agent_id_to_name(uuid: &str) -> Option<&'static str> {
    // Normalise to lower-case for a case-insensitive match.
    match uuid.to_lowercase().as_str() {
        "e370fa57-4757-3604-3648-319a48a2e3ad" => Some("Astra"),
        "569fdd95-4d10-43ab-ca70-79becc718b46" => Some("Breach"),
        "9f0d8ba9-4140-b941-57d3-a7ad57c6b417" => Some("Brimstone"),
        "22697a3d-45bf-8dd7-4fec-84a9e28c69d7" => Some("Chamber"),
        "117ed9e3-49f3-6512-3ccf-0cada7e3823b" => Some("Cypher"),
        "cc8b64c8-4b25-4ff9-6e7f-37b4da43d235" => Some("Deadlock"),
        "f94c3b30-42be-e959-889c-5aa313dba261" => Some("Fade"),
        "7f94d92c-4234-0a36-9646-3a87eb8b5eef" => Some("Gekko"),
        "95b78ed7-4637-86d9-7e41-71ba8c293152" => Some("Harbor"),
        "dade69b4-4f5a-8528-247b-219e5a1facd6" => Some("Iso"),
        "601dbbe7-43ce-be57-2a40-4abd24953621" => Some("KAYO"),
        "1dbf2edd-4729-0984-3115-daa5eed44993" => Some("Killjoy"),
        "bb2a4828-46eb-8cd1-e765-15848195d751" => Some("Neon"),
        "8e253930-4c05-31dd-1b6c-968525494517" => Some("Omen"),
        "eb93336a-449b-9c1e-0ac7-dfe9992400c1" => Some("Phoenix"),
        "0e38b510-41a8-5780-5e8f-568b2a4f2d6c" => Some("Raze"),
        "a3bfb853-43b2-7238-a4f1-ad90e9e46bcc" => Some("Reyna"),
        "6f2a04ca-43e0-be17-7f36-b3908627744d" => Some("Sage"),
        "a4af9e37-c5b2-6b37-d15c-1fd4f31946aa" => Some("Skye"),
        "320b2a48-4d9b-a075-30f1-1f93a9b638fa" => Some("Sova"),
        "1e58de9c-4950-5125-93e9-a0aee9f98746" => Some("Viper"),
        "41fb69c1-4189-7b37-f117-bcaf1e96f1bf" => Some("Vyse"),
        "5f8d3a7f-467b-97f3-062c-13acf203c006" => Some("Yoru"),
        "13ef4d4d-39c9-b513-eb7e-2eb5eb3e4832" => Some("Clove"),
        "0ea0e5ac-4e62-0ec4-38ef-d8dbfa9aff86" => Some("Tejo"),
        "add6443a-41bd-e414-f6ad-e58d267f4e95" => Some("Jett"),
        _ => None,
    }
}