use std::ffi::c_float;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::BufRead;
use std::num::ParseFloatError;
use std::path::PathBuf;

use filetime::FileTime;
use regex::Regex;
use rev_buf_reader::RevBufReader;

pub fn get_latest_ffmpeg_report_file() -> PathBuf {
    let mut log_file = None;
    let mut latest_time = FileTime::zero();

    let log_entries = get_logs_in_directory(".");

    // defining entry here so we can extend it's scope
    let mut entry: Option<&DirEntry>;
    let mut index = 0;

    while index != log_entries.len() {
        entry = log_entries.get(index);
        let file_time = FileTime::from_last_modification_time(&entry.unwrap().metadata().unwrap());
        if file_time > latest_time {
            latest_time = FileTime::from_last_modification_time(&entry.unwrap().metadata().unwrap());
            log_file = entry;
        }

        index = index + 1;
    }

    return log_file.unwrap().path();
}

pub fn extract_vmaf_score(line: &str) -> Result<c_float, ParseFloatError> {
    return capture_group(line, r"VMAF score: ([0-9]+\.[0-9]+)")
        .parse::<c_float>();
}

pub fn read_last_line_at(line_number: i32) -> String {
    let log_file = File::open(get_latest_ffmpeg_report_file()).unwrap();
    let reader = RevBufReader::new(log_file);
    let mut lines = reader.lines();

    // read from bottom to just before the line we need
    for _ in 0..line_number - 1 {
        lines.next().unwrap().unwrap();
    }

    return lines.next().unwrap().unwrap();
}

pub fn capture_group(str: &str, regex: &str) -> String {
    let re = Regex::new(regex).unwrap();
    let caps = re.captures(str);
    return if caps.is_some() {
        caps.unwrap().get(1).unwrap().as_str().to_string()
    } else {
        String::new()
    };
}

fn get_logs_in_directory(dir: &str) -> Vec<DirEntry> {
    let paths = fs::read_dir(dir).unwrap();
    return paths.filter_map(|e| e.ok())
        .filter(|p| p.file_type().unwrap().is_file())
        .collect::<Vec<DirEntry>>();
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::report_files::{extract_vmaf_score, get_logs_in_directory};

    static VMAF_LINE: &str = "[Parsed_libvmaf_0 @ 00000169cf14fc00] VMAF score: 98.644730";

    #[test]
    fn extract_vmaf_score_test() {
        let score = extract_vmaf_score(VMAF_LINE);
        assert!(score.is_ok());
        assert_eq!(score.unwrap(), 98.64473);
    }

    #[test]
    fn log_files_only_test() {
        File::create("tmp.log").expect("Unable to create temporary log file for testing");
        let log_files = get_logs_in_directory("../../");
        for file in log_files {
            assert!(file.path().extension().unwrap().to_str().unwrap().contains("log"));
        }
    }
}
