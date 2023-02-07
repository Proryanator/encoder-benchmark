use std::path::Path;

use clap::{CommandFactory, Parser};
use clap::error::ErrorKind;

use crate::DOWNLOAD_URL;

// list will grow as more permutations are supported
const SUPPORTED_ENCODERS: [&'static str; 2] = ["h264_nvenc", "hevc_nvenc"];

#[derive(Parser)]
pub(crate) struct Cli {
    /// the encoder you wish to benchmark: [h264_nvenc, hevc_nvenc, etc]
    #[arg(short, long, value_name = "encoder_name", default_value = "encoder")]
    pub(crate) encoder: String,
    /// target bitrate (in Mb/s) to output; in combination with --bitrate-max-permutation, this is the starting permutation
    #[arg(short, long, value_name = "bitrate", default_value = "10")]
    pub(crate) bitrate: u32,
    /// whether to run vmaf score on each permutation or not
    #[arg(short, long)]
    pub(crate) check_quality: bool,
    // stop an encoding session if the encoder can't keep up with the input file's FPS
    #[arg(short, long)]
    pub(crate) detect_overload: bool,
    /// the source file you wish to benchmark; if not provided, will run standard benchmark on all supported resolutions
    #[arg(short, long, value_name = "source.y4m", default_value = "")]
    pub(crate) source_file: String,
    /// runs just the first permutation for given encoder; useful for testing the tool & output
    #[arg(short, long)]
    pub(crate) test_run: bool,
    /// maximum value to increase the bitrate to (in 5Mb/s intervals); if not specified, tool will not permute over bitrate values
    #[arg(short, long, value_name = "bitrate")]
    pub(crate) max_bitrate_permutation: Option<u32>,
    /// runs through permutations that have expected duplicate scores; produces more thorough results but will add substantial runtime
    #[arg(short, long)]
    pub(crate) allow_duplicate_scores: bool,
    /// logs useful information to help troubleshooting
    #[arg(short, long)]
    pub(crate) verbose: bool,
    /// lists the supported/implemented encoders that this tool supports
    #[arg(short, long)]
    pub(crate) list_supported_encoders: bool,
}

impl Cli {
    pub(crate) fn validate(&mut self) {
        if self.list_supported_encoders {
            println!("Supported encoders: {:?}", SUPPORTED_ENCODERS);
            std::process::exit(0);
        }

        // this means no encoder was specified
        if self.encoder == "encoder" {
            let mut cmd = Cli::command();
            cmd.error(
                ErrorKind::ArgumentConflict,
                format!("Please provide one of the supported encoders via '-e encoder_name'; for a list of supported encoders use the '-l' argument"),
            ).exit();
        }

        // check if specified encoder is supported by the tool
        if !is_encoder_supported(&self.encoder) {
            let mut cmd = Cli::command();
            cmd.error(
                ErrorKind::ArgumentConflict,
                format!("[{}] is not a supported encoder at the moment", self.encoder),
            ).exit();
        }

        // check if source file exists or not
        if !self.source_file.is_empty() && !Path::new(self.source_file.as_str()).exists() {
            let mut cmd = Cli::command();
            cmd.error(
                ErrorKind::ArgumentConflict,
                format!("[{}] source file does not exist; if you want to use one of the provided source files, download them from here:\n{}", self.source_file, DOWNLOAD_URL),
            ).exit();
        }

        if self.max_bitrate_permutation.is_none() {
            self.max_bitrate_permutation = Option::from(self.bitrate);
        }
    }

    pub(crate) fn has_special_options(&self) -> bool {
        return self.check_quality || self.detect_overload || self.allow_duplicate_scores;
    }
}

fn is_encoder_supported(potential_encoder: &String) -> bool {
    return SUPPORTED_ENCODERS.contains(&potential_encoder.as_str());
}