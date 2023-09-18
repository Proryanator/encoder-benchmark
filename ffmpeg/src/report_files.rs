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
    let log_entries = get_logs_in_directory(".");
    let log_file = get_latest_log(log_entries);
    return log_file.unwrap().path();
}

pub fn extract_vmaf_score(line: &str) -> Result<c_float, ParseFloatError> {
    return capture_group(line, r"VMAF score: (\d+\.\d+)").parse::<c_float>();
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
    // Only match ffmpeg log files
    let re = Regex::new(r"^ffmpeg.*?\.log$").unwrap();
    let paths = fs::read_dir(dir).unwrap();
    return paths
        .filter_map(|e| e.ok())
        .filter(|p| {
            p.file_type().unwrap().is_file() && re.is_match(p.file_name().to_str().unwrap())
        })
        .collect::<Vec<DirEntry>>();
}

fn get_latest_log(log_entries: Vec<DirEntry>) -> Option<DirEntry> {
    let mut log_file: Option<DirEntry> = None;
    let mut latest_time = FileTime::zero();

    // defining file_time here so we can extend it's scope
    let mut file_time;

    for entry in log_entries.into_iter() {
        let metadata = entry.metadata().unwrap();
        if metadata.created().is_ok() {
            file_time = FileTime::from_system_time(metadata.created().unwrap());
        } else {
            // Platforms that don't support metadata.created() will use the
            // last modification time as a fallback
            file_time = FileTime::from_last_modification_time(&metadata);
        }

        if file_time > latest_time {
            latest_time = file_time;
            log_file = Some(entry);
        }
    }
    return log_file;
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::report_files::{extract_vmaf_score, get_latest_log, get_logs_in_directory};

    static VMAF_LINE: &str = "[Parsed_libvmaf_0 @ 00000169cf14fc00] VMAF score: 98.644730";

    #[test]
    fn extract_vmaf_score_test() {
        let score = extract_vmaf_score(VMAF_LINE);
        assert!(score.is_ok());
        assert_eq!(score.unwrap(), 98.64473);
    }

    #[test]
    fn log_files_only_test() {
        fs::create_dir("./log-test").unwrap();
        fs::File::create("./log-test/ffmpeg-1.log")
            .expect("Unable to create temporary log file for testing");
        fs::File::create("./log-test/ffmpeg-2.log")
            .expect("Unable to create temporary log file for testing");
        fs::File::create("./log-test/some-other.log")
            .expect("Unable to create temporary log file for testing");
        fs::File::create("./log-test/diff-file-ext.txt")
            .expect("Unable to create temporary log file for testing");
        let log_files = get_logs_in_directory("./log-test");
        // Only the ffmpeg*.log files should be in log_files
        assert!(log_files.len() == 2);
        for file in log_files {
            assert!(file.file_name().to_str().unwrap().contains("ffmpeg"));
            assert!(file
                .path()
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("log"));
        }

        fs::remove_dir_all("./log-test").unwrap();
    }

    #[test]
    fn latest_log_file_test() {
        fs::create_dir("./latest-log-test").expect("Unable to create temporary dir for testing");
        let file1 = fs::File::create("./latest-log-test/ffmpeg-1.log")
            .expect("Unable to create temporary log file for testing");
        file1.sync_all().unwrap();
        let file2 = fs::File::create("./latest-log-test/ffmpeg-2.log")
            .expect("Unable to create temporary log file for testing");
        file2.sync_all().unwrap();
        let log_files = get_logs_in_directory("./latest-log-test");
        let latest_log_file = get_latest_log(log_files);
        // assert latest log file name
        assert!(latest_log_file
            .unwrap()
            .file_name()
            .to_str()
            .unwrap()
            .contains("ffmpeg-2.log"));
        fs::remove_dir_all("./latest-log-test").unwrap();
    }
}
