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
    use crate::report_files::{extract_vmaf_score, get_latest_log, get_logs_in_directory};
    use std::fs;
    use std::path::Path;

    static VMAF_LINE: &str = "[Parsed_libvmaf_0 @ 00000169cf14fc00] VMAF score: 98.644730";
    static EXPECTED_DIR_CREATED_MSG: &str = "Unable to create temporary dir for testing";
    static EXPECTED_TMP_FILE_CREATED_MSG: &str = "Unable to create temporary file for testing";

    #[test]
    fn extract_vmaf_score_test() {
        let score = extract_vmaf_score(VMAF_LINE);
        assert!(score.is_ok());
        assert_eq!(score.unwrap(), 98.64473);
    }

    #[test]
    fn log_files_only_test() {
        let test_log_dir_path = Path::new("./log-test");
        let test_log_dir_path_str = test_log_dir_path.to_str().unwrap();

        if test_log_dir_path.exists() {
            fs::remove_dir_all(test_log_dir_path_str).unwrap();
        }
        fs::create_dir(test_log_dir_path_str).expect(EXPECTED_DIR_CREATED_MSG);
        let ffmpeg_log_1 = test_log_dir_path_str.to_string() + "/ffmpeg-1.log";
        fs::File::create(ffmpeg_log_1).expect(EXPECTED_TMP_FILE_CREATED_MSG);

        let ffmpeg_log_2 = test_log_dir_path_str.to_string() + "/ffmpeg-2.log";
        fs::File::create(ffmpeg_log_2).expect(EXPECTED_TMP_FILE_CREATED_MSG);
        let some_other_log = test_log_dir_path_str.to_string() + "/some-other.log";
        fs::File::create(some_other_log).expect(EXPECTED_TMP_FILE_CREATED_MSG);
        let text_file = test_log_dir_path_str.to_string() + "/diff-file-ext.txt";
        fs::File::create(text_file).expect(EXPECTED_TMP_FILE_CREATED_MSG);
        let log_files = get_logs_in_directory(test_log_dir_path_str);
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

        fs::remove_dir_all(test_log_dir_path_str).unwrap();
    }

    #[test]
    fn latest_log_file_test() {
        let test_latest_log_dir_path = Path::new("./latest-log-test");
        let test_latest_log_dir_path_str = test_latest_log_dir_path.to_str().unwrap();
        if test_latest_log_dir_path.exists() {
            fs::remove_dir_all(test_latest_log_dir_path_str).unwrap();
        }

        fs::create_dir(test_latest_log_dir_path_str).expect(EXPECTED_DIR_CREATED_MSG);
        let old_log_path_str = test_latest_log_dir_path_str.to_string() + "/ffmpeg-1.log";
        let old_log_file = fs::File::create(old_log_path_str).expect(EXPECTED_TMP_CREATED_MSG);
        old_log_file.sync_all().unwrap();

        let new_log_path_str = test_latest_log_dir_path_str.to_string() + "/ffmpeg-2.log";
        let new_log_file = fs::File::create(new_log_path_str).expect(EXPECTED_TMP_FILE_CREATED_MSG);
        new_log_file.sync_all().unwrap();
        let log_files = get_logs_in_directory(test_latest_log_dir_path_str);
        //debug
        for file in &log_files {
            println!("{:?}", file.file_name());
            println!("{:?}", &file.metadata().unwrap());
        }

        let latest_log_file = get_latest_log(log_files);
        // assert latest log file name
        assert!(latest_log_file
            .unwrap()
            .file_name()
            .to_str()
            .unwrap()
            .contains("ffmpeg-2.log"));
        fs::remove_dir_all(test_latest_log_dir_path.to_str().unwrap()).unwrap();
    }
}
