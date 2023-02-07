use std::process::{Command, Stdio};

use crate::ffmpeg::metadata::MetaData;

pub(crate) fn probe_for_video_metadata(input_file: &String) -> MetaData {
    let args = format!("-v error -select_streams v:0 -show_entries stream=duration_ts,r_frame_rate,coded_width,coded_height -of csv=p=0 {}", input_file);
    let ffprobe = Command::new("ffprobe")
        .args(args.split(" "))
        .stdout(Stdio::piped())
        .output().expect("Unable to run ffprobe to collect metadata on the input video file");

    let output = String::from_utf8_lossy(&ffprobe.stdout);
    return extract_metadata(output.to_string());
}

fn extract_metadata(line: String) -> MetaData {
    let splits = line.split(",").collect::<Vec<&str>>();

    let fps_splits = splits.get(2).unwrap().split("/").collect::<Vec<&str>>();

    let metadata = MetaData {
        fps: fps_splits.get(0).unwrap().trim().parse::<u32>().unwrap(),
        frames: splits.get(3).unwrap().to_string().trim().parse::<u64>().unwrap(),
        width: splits.get(0).unwrap().to_string().trim().parse::<u32>().unwrap(),
        height: splits.get(1).unwrap().to_string().trim().parse::<u32>().unwrap(),
    };

    return metadata;
}

#[cfg(test)]
mod tests {
    use crate::ffmpeg::ffprobe::extract_metadata;
    use crate::ffmpeg::metadata::MetaData;

    static PROBE_LINE: &str = "1920,1080,60/1,1923";

    #[test]
    fn extract_metadata_test() {
        let extracted = extract_metadata(String::from(PROBE_LINE));

        let expected = MetaData {
            fps: 60,
            frames: 1923,
            width: 1920,
            height: 1080,
        };

        assert_eq!(equals(&expected, &extracted), true);
    }

    fn equals(original: &MetaData, other: &MetaData) -> bool {
        return original.fps == other.fps
            && original.frames == other.frames
            && original.height == other.height
            && original.width == other.width;
    }
}