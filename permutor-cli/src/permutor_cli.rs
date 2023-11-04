use clap::Parser;

use cli::cli_util::{error_with_ack, standard_cli_check};

#[derive(Parser)]
pub struct PermutorCli {
    /// the encoder you wish to benchmark: [h264_nvenc, hevc_nvenc, etc]
    #[arg(short, long, value_name = "encoder_name", default_value = "encoder")]
    pub encoder: String,
    /// target bitrate (in Mb/s) to output; in combination with --bitrate-max-permutation, this is the starting permutation
    #[arg(short, long, value_name = "bitrate", default_value = "10")]
    pub bitrate: u32,
    /// whether to run vmaf score on each permutation or not
    #[arg(short, long)]
    pub check_quality: bool,
    /// when used with check_quality, encodes that produce the same quality will still be encoded
    #[arg(short, long)]
    pub(crate) allow_duplicate_scores: bool,
    // stop an encoding session if the encoder can't keep up with the input file's FPS
    #[arg(short, long)]
    pub detect_overload: bool,
    /// the source file you wish to benchmark; if not provided, will run standard benchmark on all supported resolutions
    #[arg(short, long, value_name = "source.y4m", default_value = "")]
    pub source_file: String,
    /// the directory you wish the benchmark to look for your encoder files; can be used with --source_file/-s if you wish
    #[arg(short, long, value_name = "folder/to/files", default_value = "")]
    pub files_directory: String,
    /// the directory you wish for the logs this tool produces to go into; defaults to the current directory. Note: does not change the location of temporary log files (like the ones ffmpeg makes)
    #[arg(long, value_name = "folder/to/log/output", default_value = "")]
    pub log_output_directory: String,
    /// runs just the first permutation for given encoder; useful for testing the tool & output
    #[arg(short, long)]
    pub test_run: bool,
    /// maximum value to increase the bitrate to (in 5Mb/s intervals); if not specified, tool will not permute over bitrate values
    #[arg(short, long, value_name = "bitrate")]
    pub max_bitrate_permutation: Option<u32>,
    /// logs useful information to help troubleshooting
    #[arg(short, long)]
    pub verbose: bool,
    /// lists the supported/implemented supported that this tool supports
    #[arg(short, long)]
    pub list_supported_encoders: bool,
    /// the GPU you wish to run the encode on; defaults to the first/only GPU found in your system
    #[arg(short, long, default_value = "0")]
    pub gpu: u8,
}

impl PermutorCli {
    pub fn validate(&mut self) {
        standard_cli_check(
            self.list_supported_encoders,
            &self.encoder,
            &self.source_file,
            &self.files_directory,
            false,
        );

        if self.source_file.is_empty() {
            println!("Error: No source file was provided to run on, please specify an input file");
            error_with_ack(false);
        }

        if self.max_bitrate_permutation.is_none() {
            self.max_bitrate_permutation = Option::from(self.bitrate);
        }

        if self.source_file.is_empty() && !self.files_directory.is_empty() {
            // internally map the source_file and source_files_directory together
            self.source_file = format!("{}/{}", self.files_directory, self.source_file);
        }
    }

    pub fn has_special_options(&self) -> bool {
        return self.check_quality
            || self.detect_overload
            || self.verbose
            || self.test_run
            || self.allow_duplicate_scores;
    }
}
