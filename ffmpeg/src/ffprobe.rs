use std::process::{Command, Stdio};

use crate::metadata::MetaData;

pub fn probe_for_video_metadata(input_file: &String) -> MetaData {
    // adding the input file later on, prevents the space split breaking the args
    let args = format!("-v error -select_streams v:0 -show_entries stream=duration_ts,r_frame_rate,coded_width,coded_height -of csv=p=0");
    let split_args = args.split(" ");
    let mut vec_args = split_args.collect::<Vec<&str>>();
    vec_args.push(input_file);
    let ffprobe = Command::new("ffprobe")
        .args(vec_args)
        .stdout(Stdio::piped())
        .output()
        .expect("Unable to run ffprobe to collect metadata on the input video file");

    let output = String::from_utf8_lossy(&ffprobe.stdout);
    if output.to_string().is_empty() {
        panic!("ffprobe was not able to read information on the file; check your file paths for accuracy")
    }

    return extract_metadata(output.to_string());
}

fn extract_metadata(line: String) -> MetaData {
    let splits = line.split(",").collect::<Vec<&str>>();

    let fps_splits = splits.get(2).unwrap().split("/").collect::<Vec<&str>>();

    let metadata = MetaData {
        fps: fps_splits.get(0).unwrap().trim().parse::<u32>().unwrap(),
        frames: splits
            .get(3)
            .unwrap()
            .to_string()
            .trim()
            .parse::<u64>()
            .unwrap(),
        width: splits
            .get(0)
            .unwrap()
            .to_string()
            .trim()
            .parse::<u32>()
            .unwrap(),
        height: splits
            .get(1)
            .unwrap()
            .to_string()
            .trim()
            .parse::<u32>()
            .unwrap(),
    };

    return metadata;
}

#[cfg(test)]
mod tests {
    use crate::ffprobe::extract_metadata;
    use crate::metadata::MetaData;

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
