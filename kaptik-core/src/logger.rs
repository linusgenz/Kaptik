use std::path::PathBuf;

fn log_file_path(file: &str) -> PathBuf {
    let mut dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));

    dir.push("Kaptik");
    std::fs::create_dir_all(&dir).ok();
    dir.push(file);
    dir
}

pub fn write_log(_file_name: &str, message: String) {
    #[cfg(debug_assertions)]
    {
        println!("{}", message);
        return;
    }

    #[cfg(not(debug_assertions))]
    {
        let path = log_file_path(_file_name);

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{}", message);
        }
    }
}


#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::logger::write_log(
            "logs.log",
            format!($($arg)*)
        )
    }};
}

#[macro_export]
macro_rules! ffmpeg_log {
    ($($arg:tt)*) => {{
        $crate::logger::write_log(
            "ffmpeg.log",
            format!($($arg)*)
        )
    }};
}
