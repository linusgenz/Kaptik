use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use rmpv::decode::read_value;
use rmpv::Value;
use kaptik_core::domain::game_stats::KDA;
use kaptik_core::recording_storage::{load_recording_data, RecordingData};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let path = &args[2];

    match command.as_str() {
        "info" => show_info(path),
        "events" => view_events(path),
        "event-stats" => event_stats(path),
        "timeline" => event_timeline(path),
        "apm" => view_apm(path),
        "apm-stats" => apm_stats(path),
        "apm-graph" => apm_graph(path),
        "csv-events" => export_events_csv(path),
        "csv-apm" => export_apm_csv(path),
        "tree" => dump_tree(path),
        _ => print_usage(&args[0]),
    }
}

fn print_usage(program: &str) {
    println!("Recording Viewer — unified viewer for RecordingData");
    println!();
    println!("Usage:");
    println!("  {} <command> <file.msgpack>", program);
    println!();
    println!("Commands:");
    println!("  info            Show metadata");
    println!("  events          Show events");
    println!("  event-stats     Event statistics");
    println!("  timeline        Events per time bucket");
    println!("  apm             Show APM table");
    println!("  apm-stats       APM statistics");
    println!("  apm-graph       ASCII APM graph");
    println!("  csv-events      Export events CSV");
    println!("  csv-apm         Export APM CSV");
    println!("  tree            Raw msgpack tree");
}

fn load_file(path: &str) -> RecordingData {
    match load_recording_data(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to load '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

//
// METADATA
//

fn show_info(path: &str) {
    let data = load_file(path);
    let m = &data.metadata;

    let kda = m.kda.as_ref().unwrap_or(&KDA { kills: 0, deaths: 0, assists: 0 });
    let kda_string = format!("{}/{}/{}", kda.kills, kda.deaths, kda.assists);

    println!("🎮 Recording Info");
    println!("ID: {}", m.recording_id);
    println!("Game: {}", m.game_name);
    println!("Character: {}", m.character_name.as_deref().unwrap_or("-"));
    println!("KDA: {}", kda_string);
    println!("Map: {}", m.map_name.as_deref().unwrap_or("-"));
    println!("Round: {:?}", m.round_number);
    println!("Timestamp: {}", m.timestamp);
    println!("Duration: {:.2?}s", m.duration_seconds);
    println!();
    println!("Events: {}", data.events.len());
    println!("APM points: {}", data.apm.series.len());
}

//
// EVENTS
//

fn view_events(path: &str) {
    let data = load_file(path);

    println!("🎮 Events");
    println!();

    for e in &data.events {
        println!("{:.2}s  {:?}", e.timestamp, e);
    }

    println!("\nTotal events: {}", data.events.len());
}

fn event_stats(path: &str) {
    let data = load_file(path);

    if data.events.is_empty() {
        println!("No events");
        return;
    }

    let mut per_type: HashMap<String, usize> = HashMap::new();

    for e in &data.events {
        *per_type.entry(format!("{:?}", e.event_type)).or_default() += 1;
    }

    println!("📊 Event Statistics\n");

    for (ty, count) in per_type {
        println!("{:<20} {}", ty, count);
    }
}

fn event_timeline(path: &str) {
    let data = load_file(path);

    if data.events.is_empty() {
        return;
    }

    let bucket_size = 10.0;
    let max_time = data.events.last().unwrap().timestamp;

    let bucket_count = (max_time / bucket_size).ceil() as usize + 1;
    let mut buckets = vec![0; bucket_count];

    for e in &data.events {
        let idx = (e.timestamp / bucket_size) as usize;
        buckets[idx] += 1;
    }

    println!("📈 Events per {}s", bucket_size);

    for (i, count) in buckets.iter().enumerate() {
        print!("{:>4}s │ ", i * 10);

        for _ in 0..*count {
            print!("█");
        }

        println!(" {}", count);
    }
}

//
// APM
//

fn view_apm(path: &str) {
    let data = load_file(path);

    println!("{:>8} │ {:>6}", "Time", "APM");
    println!("─────────┼────────");

    for (time, apm) in &data.apm.series {
        let minutes = (time / 60.0) as u32;
        let seconds = (time % 60.0) as u32;
        println!("{:>3}:{:02} │ {:>6}", minutes, seconds, apm);
    }
}

fn apm_stats(path: &str) {
    let data = load_file(path);

    if data.apm.series.is_empty() {
        println!("No APM data");
        return;
    }

    let values: Vec<u32> = data.apm.series.iter().map(|(_, v)| *v).collect();

    let min = values.iter().min().unwrap();
    let max = values.iter().max().unwrap();
    let avg = values.iter().sum::<u32>() as f64 / values.len() as f64;

    println!("📊 APM Stats\n");
    println!("Min: {}", min);
    println!("Max: {}", max);
    println!("Avg: {:.1}", avg);
    println!("Stored avg: {:?}", data.apm.average_apm);
    println!("Stored peak: {:?}", data.apm.peak_apm);
}

fn apm_graph(path: &str) {
    let data = load_file(path);

    if data.apm.series.is_empty() {
        return;
    }

    let max_apm = data.apm.series.iter().map(|(_, a)| *a).max().unwrap() as f64;
    let height = 20;

    for row in (0..height).rev() {
        let threshold = (row as f64 / height as f64) * max_apm;

        for (_, apm) in &data.apm.series {
            if (*apm as f64) >= threshold {
                print!("█");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}

//
// CSV
//

fn export_events_csv(path: &str) {
    let data = load_file(path);

    println!("timestamp,event");

    for e in &data.events {
        println!("{:.2},{:?}", e.timestamp, e);
    }
}

fn export_apm_csv(path: &str) {
    let data = load_file(path);

    println!("time_seconds,apm");

    for (t, apm) in &data.apm.series {
        println!("{},{}", t, apm);
    }
}

//
// RAW MSGPACK DEBUG
//

fn dump_tree(path: &str) {
    let file = File::open(path).expect("open failed");
    let mut reader = BufReader::new(file);
    let value = read_value(&mut reader).expect("invalid msgpack");

    print_value(&value, 0);
}

fn print_value(value: &Value, indent: usize) {
    let pad = " ".repeat(indent);

    match value {
        Value::Map(map) => {
            println!("{}{{", pad);
            for (k, v) in map {
                print!("{}  ", pad);
                print!("{:?}: ", k);
                print_value(v, indent + 4);
            }
            println!("{}}}", pad);
        }
        Value::Array(arr) => {
            println!("{}[", pad);
            for v in arr {
                print_value(v, indent + 4);
            }
            println!("{}]", pad);
        }
        _ => println!("{}{:?}", pad, value),
    }
}
