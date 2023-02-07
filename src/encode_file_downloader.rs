use std::fs;

use crate::ENCODE_FILES;

pub(crate) fn are_all_source_files_present() -> bool {
    let existing_video_files = get_video_files();

    for file in ENCODE_FILES {
        if !existing_video_files.contains(&String::from(file)) {
            return false;
        }
    }

    return true;
}

fn get_video_files() -> Vec<String> {
    let paths = fs::read_dir(".").unwrap();
    return paths.filter_map(|e| e.ok())
        .filter(|p| p.file_type().unwrap().is_file())
        .map(|p| p.file_name().to_str().unwrap().to_string())
        .collect::<Vec<String>>();
}