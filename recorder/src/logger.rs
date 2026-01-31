#[doc(hidden)]
#[macro_export]
macro_rules! __internal_log {
    ($file_name:expr, $($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }

        #[cfg(not(debug_assertions))]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::path::PathBuf;

            fn log_file_path(file: &str) -> PathBuf {
                let mut dir = dirs::config_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                dir.push("Kaptik");
                std::fs::create_dir_all(&dir).ok();
                dir.push(file);
                dir
            }

            let path = log_file_path($file_name);
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(file, $($arg)*);
            }
        }
    }};
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::__internal_log!("logs.log", $($arg)*);
    };
}

#[macro_export]
macro_rules! ffmpeg_log {
    ($($arg:tt)*) => {
        $crate::__internal_log!("ffmpeg.log", $($arg)*);
    };
}
