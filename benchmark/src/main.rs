use std::{env, panic};
use std::path::Path;

use clap::Parser;
use text_io::read;

use cli::cli_util::{is_dev, log_cli_header, pause};
use cli::supported::{get_supported_encoders, get_supported_inputs};
use codecs::amf::Amf;
use codecs::apple_silicon::Apple;
use codecs::av1_qsv::AV1QSV;
use codecs::get_vendor_for_codec;
use codecs::nvenc::Nvenc;
use codecs::permute::Permute;
use codecs::qsv::QSV;
use codecs::vendor::Vendor;
use engine::benchmark_engine::BenchmarkEngine;
use ffmpeg::metadata::MetaData;
use gpus::get_gpus;
use permutation::permutation::Permutation;

use crate::benchmark_cli::BenchmarkCli;

mod benchmark_cli;

fn main() {
    let result = panic::catch_unwind(|| {
        benchmark();
    });

    if result.is_err() {
        eprintln!("Unhandled error encountered, see panic errors above...");
    }

    pause();
}

fn benchmark() {
    log_cli_header(String::from("Encoder Benchmark"));
    let mut cli = BenchmarkCli::new();

    // check how many Nvidia GPU's there are
    // eventually will add support for checking AMD/Intel GPU's too
    let gpus = get_gpus();

    // if no args were provided, they will be prompted from the user
    // this works for both cli running as well as just clicking the executable
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        read_user_input(&mut cli, gpus);
        cli.set_ui_opened();
    } else {
        cli = BenchmarkCli::parse();
    }

    cli.validate();

    let input_files = get_input_files(cli.source_file.clone(), cli.files_directory.clone());
    let mut engine = BenchmarkEngine::new(cli.log_output_directory.clone());

    // prepare permutations for the engine to run over
    for input in input_files {
        let mut permutation = Permutation::new(input, cli.encoder.clone());
        let settings = get_benchmark_settings_for(&cli);
        let bitrate = get_bitrate_for(&permutation.get_metadata(), cli.encoder.clone());

        permutation.bitrate = bitrate;
        permutation.encoder_settings = settings;
        permutation.verbose = cli.verbose;

        // tell this encode run that we'll want to preserve the file output
        if cli.decode {
            permutation.is_decoding = true;
        }

        engine.add(permutation.clone());

        if cli.decode {
            let mut decode_permutation = permutation.clone();
            decode_permutation.decode_run = true;
            engine.add(decode_permutation);
        }
    }

    engine.run();
    pause();
}

fn read_user_input(cli: &mut BenchmarkCli, gpus: Vec<String>) {
    // if more than 1 GPU is identified, ask for the user to choose which one
    if gpus.len() > 1 {
        loop {
            let str_vec = gpus.iter().map(|s| &**s).collect();
            print_options(str_vec);
            print!("Choose GPU [0-{}]: ", gpus.len() - 1);
            let input: String = read!("{}");

            if !is_numeric(&input) {
                println!("Invalid input, try again...")
            } else {
                let value: u8 = input.parse().unwrap();

                if value as usize >= gpus.len() {
                    println!("Invalid input, try again...");
                } else {
                    cli.gpu = value;
                    println!();
                    break;
                }
            }
        }
    }

    loop {
        print_options(get_supported_encoders().to_vec());
        print!(
            "Choose encoder [0-{}]: ",
            get_supported_encoders().len() - 1
        );
        let input: String = read!("{}");

        if !is_numeric(&input) {
            println!("Invalid input, try again...")
        } else {
            let value: usize = input.parse().unwrap();

            if value >= get_supported_encoders().len() {
                println!("Invalid input, try again...");
            } else {
                cli.encoder = String::from(get_supported_encoders()[value]);
                break;
            }
        }
    }

    loop {
        print!("\nRun decode benchmark along with encode benchmark? [y/n]: ");
        let full: String = read!("{}");
        if full != "n" && full != "y" {
            println!("Invalid input, try again...");
        } else {
            if full == "y" {
                cli.decode = true;
            }

            break;
        }
    }

    let mut full_bench = false;
    loop {
        print!("\nRun full benchmark? [y/n]: ");
        let full: String = read!("{}");
        if full != "n" && full != "y" {
            println!("Invalid input, try again...");
        } else {
            if full == "y" {
                full_bench = true;
            }

            break;
        }
    }

    // user may specify a directory for the source files
    let mut in_current_dir = false;
    loop {
        print!("\nAre the source files in the current directory? [y/n]: ");
        let in_current_directory: String = read!("{}");
        if in_current_directory != "n" && in_current_directory != "y" {
            println!("Invalid input, try again...");
        } else {
            if in_current_directory == "y" {
                in_current_dir = true;
            }

            break;
        }

        break;
    }

    if !in_current_dir {
        loop {
            print!("\nPlease specify the input source file directory: ");
            let source_files_directory: String = read!("{}");

            if !(Path::new(source_files_directory.as_str()).exists()) {
                print!("The provided directory does not exist, please check your input and try again...")
            } else {
                cli.files_directory = source_files_directory;
                break;
            }
        }
    }

    if !full_bench {
        loop {
            print_options(get_supported_inputs().to_vec());
            print!(
                "\nChoose video file to encode [0-{}]: ",
                get_supported_inputs().len() - 1
            );
            let input: String = read!("{}");
            if !is_numeric(&input) {
                println!("Invalid input, try again...");
            } else {
                let value: usize = input.parse().unwrap();
                if value >= get_supported_inputs().len() {
                    println!("Invalid input, try again...");
                } else {
                    cli.source_file = String::from(get_supported_inputs()[value]);
                    break;
                }
            }
        }
    }

    loop {
        print!("\nRun with verbose mode? [y/n]: ");
        let full: String = read!("{}");
        if full != "n" && full != "y" {
            println!("Invalid input, try again...");
        } else {
            if full == "y" {
                cli.verbose = true;
            }

            break;
        }
    }

    println!();
}

fn is_numeric(input: &String) -> bool {
    return input.chars().all(char::is_numeric);
}

fn print_options(input_vec: Vec<&str>) {
    for i in 0..input_vec.len() {
        println!("[{}] - {}", i, input_vec[i]);
    }
}

fn get_benchmark_settings_for(cli: &BenchmarkCli) -> String {
    let vendor = get_vendor_for_codec(&cli.encoder);

    return match vendor {
        Vendor::Nvidia => {
            let nvenc = Nvenc::new(cli.encoder == "hevc_nvenc", cli.gpu);
            nvenc.get_benchmark_settings()
        }

        Vendor::AMD => {
            let amf = Amf::new(cli.encoder == "hevc_amf", cli.gpu);
            amf.get_benchmark_settings()
        }

        Vendor::IntelQSV => {
            if cli.encoder.contains("av1") {
                let intel_av1 = AV1QSV::new();
                intel_av1.get_benchmark_settings()
            } else {
                let intel_qsv = QSV::new(cli.encoder == "hevc_qsv");
                intel_qsv.get_benchmark_settings()
            }
        }
        Vendor::Apple => {
            if cli.encoder.contains("h264") {
                let apple = Apple::new(true, false);
                apple.get_benchmark_settings()
            } else if cli.encoder.contains("hevc") {
                let apple_h264 = Apple::new(false, false);
                apple_h264.get_benchmark_settings()
            } else {
                let apple_h264 = Apple::new(false, true);
                apple_h264.get_benchmark_settings()
            }
        }
        Vendor::Unknown => {
            // nothing to do here
            String::from("")
        }
    };
}

fn get_bitrate_for(metadata: &MetaData, string: String) -> u32 {
    if string.contains("nvenc") {
        return *Nvenc::get_resolution_to_bitrate_map(metadata.fps)
            .get(&metadata.get_res())
            .unwrap();
    } else {
        return *Amf::get_resolution_to_bitrate_map(metadata.fps)
            .get(&metadata.get_res())
            .unwrap();
    }
}

fn get_input_files(source_file: String, source_files_directory: String) -> Vec<String> {
    if source_file.is_empty() {
        return get_supported_inputs()
            .iter()
            .map(|s| map_file(is_dev(), source_files_directory.clone(), s))
            .collect::<Vec<String>>();
    }

    return vec![source_file];
}

fn map_file(is_dev: bool, source_files_directory: String, s: &&str) -> String {
    let mut file = String::new();
    if !source_files_directory.is_empty() {
        file.push_str(format!("{}/{}", source_files_directory, *s).as_str());
    } else if is_dev {
        file.push_str("../");
        file.push_str(*s);
    } else {
        file.push_str(*s);
    }

    return file;
}
