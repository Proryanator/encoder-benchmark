use std::ffi::c_float;

pub static TCP_LISTEN: &str = "tcp://localhost:2000?listen";
pub static NO_OUTPUT: &str = "-f null -";

#[derive(Clone)]
pub struct FfmpegArgs {
    fps_limit: u32,
    report: bool,
    send_progress: bool,
    first_input: String,
    second_input: String,
    pub bitrate: u32,
    pub encoder: String,
    pub encoder_args: String,
    pub output_args: String,
    pub is_vmaf: bool,
    pub stats_period: c_float,
}

impl Default for FfmpegArgs {
    fn default() -> Self {
        FfmpegArgs {
            fps_limit: 0,
            report: false,
            send_progress: true,
            first_input: String::new(),
            second_input: String::new(),
            bitrate: u32::default(),
            encoder: String::new(),
            encoder_args: String::new(),
            output_args: NO_OUTPUT.to_string(),
            is_vmaf: false,
            // the lower the value, the more often the progress bar will update
            // but your fps calculations might be a little over-inflated
            stats_period: 0.5,
        }
    }
}

impl FfmpegArgs {
    pub fn build_ffmpeg_args(first_input: String, encoder: String, encoder_args: &String, current_bitrate: u32) -> FfmpegArgs {
        let ffmpeg_args = FfmpegArgs {
            first_input,
            bitrate: current_bitrate,
            encoder,
            encoder_args: encoder_args.to_string(),
            ..Default::default()
        };

        return ffmpeg_args;
    }

    pub fn map_to_vmaf(&self, fps: u32) -> FfmpegArgs {
        let mut vmaf_args = self.clone();

        // required for having high fps inputs score correctly
        vmaf_args.fps_limit = fps;
        vmaf_args.second_input = self.first_input.clone();
        vmaf_args.first_input = String::from(TCP_LISTEN);
        vmaf_args.output_args = String::from(NO_OUTPUT);
        vmaf_args.is_vmaf = true;
        vmaf_args.send_progress = false;
        // vmaf needs to report so we can get the vmaf score
        vmaf_args.report = true;

        return vmaf_args;
    }

    pub fn to_string(&self) -> String {
        let mut output = String::new();

        // not all will want to send progress
        if self.send_progress {
            output.push_str(format!("-progress tcp://localhost:1234 -stats_period {} ", self.stats_period).as_str());
        }

        if self.report {
            output.push_str("-report ");
        }

        if self.fps_limit != 0 {
            output.push_str(format!("-r {} ", self.fps_limit).as_str());
        }

        output.push_str(["-i", self.first_input.as_str()].join(" ").as_str());

        if !self.second_input.is_empty() {
            if self.fps_limit != 0 {
                output.push_str(format!(" -r {}", self.fps_limit).as_str());
            }

            output.push_str([" -i", self.second_input.as_str()].join(" ").as_str());
        }

        if self.is_vmaf {
            append_vmaf_only_args(&mut output);
        } else {
            append_encode_only_args(&mut output, self.bitrate, &self.encoder, &self.encoder_args);
        }

        output.push(' ');
        output.push_str(self.output_args.as_str());

        return output;
    }

    pub fn set_no_output_for_error(&mut self) {
        self.output_args = NO_OUTPUT.to_string();
        self.send_progress = false;
    }

    pub fn to_vec(&self) -> Vec<String> {
        return self.to_string().split(" ").map(|s| s.to_string()).collect();
    }
}

fn append_encode_only_args(arg_str: &mut String, bitrate: u32, encoder: &String, encoder_args: &String) {
    arg_str.push_str([" -b:v", bitrate.to_string().as_str()].join(" ").as_str());
    // adding the rate amount to the end of the bitrate
    arg_str.push('M');
    arg_str.push_str([" -c:v", encoder.as_str()].join(" ").as_str());
    arg_str.push(' ');
    arg_str.push_str(encoder_args.as_str());
}

fn append_vmaf_only_args(arg_str: &mut String) {
    arg_str.push_str(format!(" -filter_complex libvmaf='n_threads={}:n_subsample=5'", num_cpus::get()).as_str());
}

// TODO: get rid of this later
pub struct Cli {
    pub encoder: String,
    pub bitrate: u32,
    pub check_quality: bool,
    pub detect_overload: bool,
    pub source_file: String,
    pub test_run: bool,
    pub max_bitrate_permutation: Option<u32>,
    pub allow_duplicate_scores: bool,
    pub verbose: bool,
    pub list_supported_encoders: bool,
}

#[cfg(test)]
mod tests {
    use crate::args::{Cli, FfmpegArgs, NO_OUTPUT, TCP_LISTEN};

    static INPUT_ONE: &str = "1080-60.y4m";
    static INPUT_TWO: &str = "1080-60-2.y4m";
    static BITRATE: u32 = 6;
    static FPS_LIMIT: u32 = 60;
    static ENCODER: &str = "h264_nvenc";
    static ENCODER_ARGS: &str = "-preset hq -tune hq -profile:v high -rc cbr -multipass qres -rc-lookahead 8";

    #[test]
    fn default_args_test() {
        let args = FfmpegArgs::default();

        // check fields that have defaults
        assert_eq!(args.fps_limit, 0);
        assert_eq!(args.send_progress, true);
        assert_eq!(args.report, false);
        assert_eq!(args.bitrate, u32::default());
        assert_eq!(args.output_args, "-f null -");
        assert_eq!(args.is_vmaf, false);
        assert_eq!(args.stats_period, 0.5);

        // check fields that do not
        assert!(args.first_input.is_empty());
        assert!(args.second_input.is_empty());
        assert!(args.encoder.is_empty());
        assert!(args.encoder_args.is_empty());
    }

    #[test]
    fn build_all_args_test() {
        let ffmpeg_args = get_two_input_args();

        assert_eq!(ffmpeg_args.first_input, INPUT_ONE);
        assert_eq!(ffmpeg_args.second_input, INPUT_TWO);
        assert_eq!(ffmpeg_args.bitrate, BITRATE);
        assert_eq!(ffmpeg_args.encoder, ENCODER);
        assert_eq!(ffmpeg_args.encoder_args, ENCODER_ARGS);
    }

    #[test]
    fn build_only_one_input_test() {
        let ffmpeg_args = get_one_input_args();

        assert_eq!(ffmpeg_args.first_input, INPUT_ONE);
        assert_eq!(ffmpeg_args.second_input, "");
        assert_eq!(ffmpeg_args.bitrate, BITRATE);
        assert_eq!(ffmpeg_args.encoder, ENCODER);
        assert_eq!(ffmpeg_args.encoder_args, ENCODER_ARGS);
    }

    #[test]
    fn to_string_one_input_test() {
        assert_eq!(get_one_input_args().to_string(),
                   "-progress tcp://localhost:1234 -stats_period 0.5 -i 1080-60.y4m -b:v 6M -c:v h264_nvenc -preset hq -tune hq -profile:v high -rc cbr -multipass qres -rc-lookahead 8 -f null -"
        );
    }

    #[test]
    fn to_string_two_input_test() {
        assert_eq!(get_two_input_args().to_string(),
                   "-progress tcp://localhost:1234 -stats_period 0.5 -i 1080-60.y4m -i 1080-60-2.y4m -b:v 6M -c:v h264_nvenc -preset hq -tune hq -profile:v high -rc cbr -multipass qres -rc-lookahead 8 -f null -"
        );
    }

    #[test]
    fn map_to_vmaf_test() {
        let args = get_two_input_args();
        let vmaf_args = args.map_to_vmaf(FPS_LIMIT);

        assert_eq!(vmaf_args.fps_limit, FPS_LIMIT);
        assert_eq!(vmaf_args.first_input, String::from(TCP_LISTEN));
        assert_eq!(vmaf_args.second_input, args.first_input);
        assert_eq!(vmaf_args.output_args, String::from(NO_OUTPUT));
        assert_eq!(vmaf_args.is_vmaf, true);
        assert_eq!(vmaf_args.send_progress, false);
        assert_eq!(vmaf_args.report, true);
    }

    #[test]
    fn map_to_vmaf_to_string_test() {
        let vmaf_args = get_two_input_args().map_to_vmaf(FPS_LIMIT);
        assert_eq!(vmaf_args.to_string(),
                   format!("-report -r {} -i tcp://localhost:2000?listen -r {} -i 1080-60.y4m -filter_complex libvmaf='n_threads={}:n_subsample=5' -f null -", FPS_LIMIT, FPS_LIMIT, num_cpus::get().to_string())
        );
    }

    fn get_one_input_args() -> FfmpegArgs {
        let args = Cli {
            encoder: ENCODER.to_string(),
            bitrate: BITRATE,
            check_quality: false,
            detect_overload: false,
            source_file: INPUT_ONE.to_string(),
            test_run: false,
            max_bitrate_permutation: None,
            allow_duplicate_scores: false,
            verbose: false,
            list_supported_encoders: false,
        };

        return FfmpegArgs::build_ffmpeg_args(args.source_file, args.encoder, &ENCODER_ARGS.to_string(), args.bitrate);
    }

    fn get_two_input_args() -> FfmpegArgs {
        let mut args = get_one_input_args();
        args.second_input = INPUT_TWO.to_string();
        return args;
    }
}