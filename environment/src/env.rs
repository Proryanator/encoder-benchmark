use std::process::{Command, Stdio};

pub fn fail_if_environment_not_setup() {
    if !is_ffmpeg_installed() {
        println!("ffmpeg is either not installed or not setup on your path correctly");
        std::process::exit(1);
    }

    if !is_ffprobe_installed() {
        println!("ffprobe is either not installed or not setup on your path correctly");
        std::process::exit(1);
    }
}

fn is_ffmpeg_installed() -> bool {
    return is_installed("ffmpeg");
}

fn is_ffprobe_installed() -> bool {
    return is_installed("ffprobe");
}

fn is_installed(program: &str) -> bool {
    return Command::new(program)
        // important cause we don't want the help message to output here
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn().is_ok();
}