use std::env;
use std::path::PathBuf;

use recorder::recorder::apm::{APMData, load_apm_msgpack};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "view" | "show" => {
            if args.len() < 3 {
                eprintln!("Error: Missing file path");
                print_usage(&args[0]);
                std::process::exit(1);
            }
            view_apm(&args[2]);
        }
        "stats" => {
            if args.len() < 3 {
                eprintln!("Error: Missing file path");
                print_usage(&args[0]);
                std::process::exit(1);
            }
            show_stats(&args[2]);
        }
        "graph" => {
            if args.len() < 3 {
                eprintln!("Error: Missing file path");
                print_usage(&args[0]);
                std::process::exit(1);
            }
            show_graph(&args[2]);
        }
        "csv" => {
            if args.len() < 3 {
                eprintln!("Error: Missing file path");
                print_usage(&args[0]);
                std::process::exit(1);
            }
            let output_file = args.get(3).map(|s| s.as_str());
            export_csv(&args[2], output_file);
        }
        "help" | "--help" | "-h" => {
            print_usage(&args[0]);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage(&args[0]);
            std::process::exit(1);
        }
    }
}

fn print_usage(program: &str) {
    println!("APM Viewer - View and analyze Actions Per Minute data");
    println!();
    println!("Usage:");
    println!("  {} <command> <file.apm>", program);
    println!();
    println!("Commands:");
    println!("  view <file>    Show raw APM data as table");
    println!("  stats <file>   Show statistics (min, max, average, etc.)");
    println!("  graph <file>   Show ASCII graph of APM over time");
    println!("  csv <file>     Export as CSV format");
    println!("  help           Show this help message");
    println!();
    println!("Examples:");
    println!("  {} view recording.apm", program);
    println!("  {} stats recording.apm", program);
    println!("  {} graph recording.apm", program);
    println!("  {} csv recording.apm > output.csv", program);
}

fn load_file(path: &str) -> APMData {
    match load_apm_msgpack(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error loading file '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn view_apm(path: &str) {
    let data = load_file(path);

    println!("📊 APM Data from: {}", path);
    println!();
    println!("{:>8} │ {:>6}", "Time", "APM");
    println!("─────────┼────────");

    for (time, apm) in &data.series {
        let minutes = (time / 60.0) as u32;
        let seconds = (time % 60.0) as u32;
        println!("{:>3}:{:02} │ {:>6}", minutes, seconds, apm);
    }

    println!();
    println!("Total data points: {}", data.series.len());
}

fn show_stats(path: &str) {
    let data = load_file(path);

    if data.series.is_empty() {
        println!("No data in file");
        return;
    }

    let apms: Vec<u32> = data.series.iter().map(|(_, apm)| *apm).collect();
    let total_time = data.series.last().map(|(t, _)| *t).unwrap_or(0.0);

    let min_apm = *apms.iter().min().unwrap();
    let max_apm = *apms.iter().max().unwrap();
    let avg_apm = apms.iter().sum::<u32>() as f64 / apms.len() as f64;

    // Calculate median
    let mut sorted_apms = apms.clone();
    sorted_apms.sort_unstable();
    let median_apm = if sorted_apms.len() % 2 == 0 {
        let mid = sorted_apms.len() / 2;
        (sorted_apms[mid - 1] + sorted_apms[mid]) as f64 / 2.0
    } else {
        sorted_apms[sorted_apms.len() / 2] as f64
    };

    println!("📊 APM Statistics for: {}", path);
    println!();
    println!(
        "Duration:     {:>6.1}s ({} min {}s)",
        total_time,
        (total_time / 60.0) as u32,
        (total_time % 60.0) as u32
    );
    println!("Data points:  {:>6}", data.series.len());
    println!();
    println!("Min APM:      {:>6}", min_apm);
    println!("Max APM:      {:>6}", max_apm);
    println!("Average APM:  {:>6.1}", avg_apm);
    println!("Median APM:   {:>6.1}", median_apm);
    println!();

    // Find peak period
    if let Some((peak_time, peak_apm)) = data.series.iter().max_by_key(|(_, apm)| apm) {
        let minutes = (peak_time / 60.0) as u32;
        let seconds = (peak_time % 60.0) as u32;
        println!("Peak APM:     {} at {}:{:02}", peak_apm, minutes, seconds);
    }
}

fn show_graph(path: &str) {
    let data = load_file(path);

    if data.series.is_empty() {
        println!("No data in file");
        return;
    }

    println!("📈 APM Graph: {}", path);
    println!();

    let max_apm = data.series.iter().map(|(_, apm)| *apm).max().unwrap() as f64;
    let graph_height = 20;
    let graph_width = 60;

    // Group data into buckets for the graph width
    let bucket_size = (data.series.len() as f64 / graph_width as f64).ceil() as usize;
    let mut buckets = vec![];

    for chunk in data.series.chunks(bucket_size.max(1)) {
        let avg = chunk.iter().map(|(_, apm)| *apm).sum::<u32>() as f64 / chunk.len() as f64;
        buckets.push(avg);
    }

    // Draw graph from top to bottom
    for row in (0..graph_height).rev() {
        let threshold = (row as f64 / graph_height as f64) * max_apm;

        print!("{:4.0} │", threshold);

        for &value in &buckets {
            if value >= threshold {
                print!("█");
            } else if value >= threshold - (max_apm / graph_height as f64) / 2.0 {
                print!("▄");
            } else {
                print!(" ");
            }
        }
        println!();
    }

    print!("     └");
    for _ in 0..buckets.len() {
        print!("─");
    }
    println!();

    let total_time = data.series.last().map(|(t, _)| *t).unwrap_or(0.0);
    println!(
        "       0s{:>width$}",
        format!("{}s", total_time as u32),
        width = buckets.len().saturating_sub(2)
    );
}

fn export_csv(path: &str, output: Option<&str>) {
    let data = load_file(path);

    let csv_content = generate_csv(&data);

    if let Some(output_path) = output {
        match std::fs::write(output_path, csv_content) {
            Ok(_) => {
                eprintln!("✅ CSV exported to: {}", output_path);
            }
            Err(e) => {
                eprintln!("❌ Failed to write CSV file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        print!("{}", csv_content);
    }
}

fn generate_csv(data: &APMData) -> String {
    let mut output = String::from("time_seconds,time_formatted,apm\n");

    for (time, apm) in &data.series {
        let minutes = (time / 60.0) as u32;
        let seconds = (time % 60.0) as u32;
        output.push_str(&format!("{:.2},{:02}:{:02},{}\n", time, minutes, seconds, apm));
    }

    output
}