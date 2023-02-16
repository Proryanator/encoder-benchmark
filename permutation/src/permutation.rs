use ffmpeg::ffprobe::probe_for_video_metadata;
use ffmpeg::metadata::MetaData;

#[derive(Clone)]
pub struct Permutation {
    pub video_file: String,
    pub encoder: String,
    pub encoder_settings: String,
    pub bitrate: u32,
    pub metadata: MetaData,
    pub check_quality: bool,
    pub detect_overload: bool,
    pub verbose: bool,
}

impl Permutation {
    pub fn new(video_file: String, encoder: String) -> Self {
        Self {
            video_file,
            encoder,
            encoder_settings: String::from(""),
            bitrate: 0,
            metadata: MetaData::new(),
            check_quality: false,
            detect_overload: false,
            verbose: false,
        }
    }

    pub fn get_metadata(&mut self) -> MetaData {
        if self.metadata.is_empty() {
            self.metadata = probe_for_video_metadata(&self.video_file);
        }

        return self.metadata;
    }
}