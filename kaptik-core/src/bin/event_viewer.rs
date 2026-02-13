use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use rmpv::Value;
use rmpv::decode::read_value;
use kaptik_core::game_integration::event_storage::{load_events_msgpack, RecordingEvents};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "view" => view_events(&args[2]),
        "stats" => show_stats(&args[2]),
        "timeline" => show_timeline(&args[2]),
        "csv" => export_csv(&args[2]),
        "tree" => dump_tree(&args[2]),
            _ => print_usage(&args[0]),
    }
}

fn print_usage(program: &str) {
    println!("Event Viewer - View and analyze game event recordings");
    println!();

    println!("Usage:");
    println!("  {} <command> <file.events>", program);
    println!();

    println!("Commands:");
    println!("  view <file>        Show all events as table");
    println!("  stats <file>       Show event statistics and counts");
    println!("  timeline <file>    Show events per time bucket");
    println!("  highlights <file>  Show highlight events only");
    println!("  csv <file>         Export events as CSV");
    println!("  help               Show this help message");
    println!();

    println!("Examples:");
    println!("  {} view match.events", program);
    println!("  {} stats match.events", program);
    println!("  {} highlights match.events", program);
    println!("  {} timeline match.events", program);
    println!("  {} csv match.events > output.csv", program);
}


fn load_file(path: &str) -> RecordingEvents {
    match load_events_msgpack(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error loading '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn view_events(path: &str) {
    let data = load_file(path);

    println!("🎮 Events: {}", data.game_name);
    println!("Recording: {}", data.recording_id);
    println!();

    const TIME_W: usize = 6;
    const TYPE_W: usize = 12;
    const NAME_W: usize = 20;
    const MARK_W: usize = 1;

    println!(
        "{:>TIME_W$} │ {:>TYPE_W$} │ {:<NAME_W$} │ {:^MARK_W$} │ Actor → Target",
        "Time",
        "Type",
        "Name",
        "H",
    );

    println!(
        "{}┼{}┼{}┼{}┼────────────────",
        "─".repeat(TIME_W + 1),
        "─".repeat(TYPE_W + 2),
        "─".repeat(NAME_W + 2),
        "─".repeat(MARK_W + 2),
    );

    for e in &data.events {
        let minutes = (e.timestamp / 60.0) as u32;
        let seconds = (e.timestamp % 60.0) as u32;

        let time = format!("{:>3}:{:02}", minutes, seconds);
        let event_type = format!("{:?}", e.event_type);

        println!(
            "{:>TIME_W$} │ {:>TYPE_W$} │ {:<NAME_W$} │ {} → {}",
            time,
            event_type,
            e.data.name,
            e.data.actor.as_deref().unwrap_or("-"),
            e.data.target.as_deref().unwrap_or("-"),
        );

    }

    println!();
    println!("Total events: {}", data.events.len());
}

fn show_stats(path: &str) {
    let data = load_file(path);

    if data.events.is_empty() {
        println!("No events");
        return;
    }

    let mut per_type: HashMap<String, usize> = HashMap::new();

    for e in &data.events {
        *per_type.entry(format!("{:?}", e.event_type)).or_default() += 1;
    }

    println!("📊 Event Statistics");
    println!();

    println!("Game: {}", data.game_name);
    println!("Events: {}", data.events.len());

    println!("\nEvents by type:");
    for (ty, count) in per_type {
        println!("  {:<12} {}", ty, count);
    }

    if let Some(last) = data.events.last() {
        println!("\nDuration: {:.1}s", last.timestamp);
    }
}

fn show_timeline(path: &str) {
    let data = load_file(path);

    if data.events.is_empty() {
        return;
    }

    let bucket_size = 10.0; // seconds
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

fn export_csv(path: &str) {
    let data = load_file(path);

    println!("timestamp,event_type,name,actor,target,highlight");

    for e in &data.events {
        println!(
            "{:.2},{:?},{},{},{}",
            e.timestamp,
            e.event_type,
            e.data.name,
            e.data.actor.as_deref().unwrap_or(""),
            e.data.target.as_deref().unwrap_or(""),
        );
    }
}

fn dump_tree(path: &str) {
    let file = File::open(path).expect("Failed to open file");
    let mut reader = BufReader::new(file);

    let value = read_value(&mut reader).expect("Invalid msgpack");

    println!("📦 MessagePack tree:\n");
    print_value(&value, 0);
}

fn print_value(value: &Value, indent: usize) {
    let pad = " ".repeat(indent);

    match value {
        Value::Map(map) => {
            println!("{}{{", pad);

            for (k, v) in map {
                print!("{}  ", pad);
                print_value_inline(k);
                print!(": ");
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

        _ => {
            print!("{}", pad);
            print_value_inline(value);
            println!();
        }
    }
}

fn print_value_inline(value: &Value) {
    match value {
        Value::String(s) => print!("\"{}\"", s),
        Value::Integer(i) => print!("{}", i),
        Value::Boolean(b) => print!("{}", b),
        Value::F32(f) => print!("{}", f),
        Value::F64(f) => print!("{}", f),
        Value::Nil => print!("null"),
        other => print!("{:?}", other),
    }
}
