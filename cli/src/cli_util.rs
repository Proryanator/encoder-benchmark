use std::path::Path;
use std::{env, fs};

use environment::env::fail_if_environment_not_setup;

use crate::supported::{
    get_download_url, get_supported_encoders, get_supported_inputs, is_encoder_supported,
};

pub fn is_dev() -> bool {
    let args: Vec<String> = env::args().collect();
    return args[0].contains("target");
}

pub fn get_video_files() -> Vec<String> {
    let locale = if is_dev() { "../" } else { "." };

    let paths = fs::read_dir(locale).unwrap();
    return paths
        .filter_map(|e| e.ok())
        .filter(|p| p.file_type().unwrap().is_file())
        .map(|p| p.file_name().to_str().unwrap().to_string())
        .collect::<Vec<String>>();
}

pub fn are_all_source_files_present() -> bool {
    let existing_video_files = get_video_files();

    for file in get_supported_inputs() {
        if !existing_video_files.contains(&String::from(file)) {
            return false;
        }
    }

    return true;
}

pub fn standard_cli_check(
    show_encoders: bool,
    encoder: &String,
    source_file: &String,
    was_ui_opened: bool,
) {
    fail_if_environment_not_setup();

    if show_encoders {
        println!("Supported supported: {:?}", get_supported_encoders());
        dont_disappear::any_key_to_continue::default();
        std::process::exit(0);
    }

    // this means no encoder was specified
    if encoder == "encoder" {
        println!("Error: Please provide one of the supported supported via '-e encoder_name'; for a list of supported supported use the '-l' argument");
        error_with_ack(was_ui_opened);
    }

    // check if specified encoder is supported by the tool
    if !is_encoder_supported(&encoder) {
        println!(
            "Error: [{}] is not a supported encoder at the moment",
            encoder
        );
        error_with_ack(was_ui_opened);
    }

    // check if source file exists or not
    if !source_file.is_empty() && !Path::new(source_file.as_str()).exists() {
        println!("Error: [{}] source file does not exist; if you want to use one of the provided source files, download them from here:\n{}", source_file, get_download_url());
        error_with_ack(was_ui_opened);
    }
}

pub fn error_with_ack(ack: bool) {
    // want to give the user a chance to acknowledge the error
    if ack {
        dont_disappear::any_key_to_continue::custom_msg("Press any key to close the program...");
    }

    std::process::exit(1);
}

pub fn pause() {
    dont_disappear::any_key_to_continue::custom_msg("Press any key to close the program...");
}
