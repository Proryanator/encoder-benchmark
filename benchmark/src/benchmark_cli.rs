use clap::Parser;

use cli::cli_util::{are_all_source_files_present, error_with_ack, standard_cli_check};
use cli::supported::{get_download_url, get_supported_inputs};

#[derive(Parser)]
pub struct BenchmarkCli {
    /// lists the supported/implemented supported that this tool supports
    #[arg(short, long)]
    pub list_supported_encoders: bool,
    /// the encoder you wish to benchmark: [h264_nvenc, hevc_nvenc, etc]
    #[arg(short, long, value_name = "encoder_name", default_value = "encoder")]
    pub encoder: String,
    /// the source file you wish to benchmark; if not provided, will run standard benchmark on all supported resolutions
    #[arg(short, long, value_name = "source.y4m", default_value = "")]
    pub source_file: String,
    /// logs useful information to help troubleshooting
    #[arg(short, long)]
    pub verbose: bool,
    was_ui_opened: bool,
}

impl BenchmarkCli {
    pub fn set_ui_opened(&mut self) {
        self.was_ui_opened = true;
    }

    // used when taking user input for the benchmark
    pub fn new() -> Self {
        return Self {
            list_supported_encoders: false,
            encoder: String::from(""),
            source_file: String::from(""),
            verbose: false,
            was_ui_opened: false,
        };
    }

    pub fn validate(&mut self) {
        standard_cli_check(self.list_supported_encoders, &self.encoder, &self.source_file, self.was_ui_opened);

        // if you did not provide a source file, we'll be running on all expected files
        if self.source_file.is_empty() && !are_all_source_files_present() {
            println!("You're missing some video source files to run the standard benchmark; you should have the following: \n{:?}", get_supported_inputs());
            println!("Please download the ones you are missing from: {}", get_download_url());
            println!("If you want to run the tool against a specific resolution/fps, download just that source file and specify it with '-s'");
            error_with_ack(self.was_ui_opened);
        }
    }
}