use std::{env, fs};
use std::fs::File;
use std::io::Write;

use clap::Parser;
use figlet_rs::FIGfont;
use text_io::read;

use cli::cli_util::{is_dev, pause};
use cli::supported::{get_supported_encoders, get_supported_inputs};
use codecs::amf::Amf;
use codecs::get_vendor_for_codec;
use codecs::intel_igpu::IntelIGPU;
use codecs::nvenc::Nvenc;
use codecs::permute::Permute;
use codecs::vendor::Vendor;
use engine::benchmark_engine::BenchmarkEngine;
use ffmpeg::metadata::MetaData;
use gpus::get_gpus;
use permutation::permutation::Permutation;

use crate::benchmark_cli::BenchmarkCli;

mod benchmark_cli;

fn main() {
    let small_font = include_str!("small.flf");

    fig_title(String::from("Encoder-Benchmark"), String::from(small_font));
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

    let input_files = get_input_files(cli.source_file.clone());
    let mut engine = BenchmarkEngine::new();
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
        print!("Choose encoder [0-{}]: ", get_supported_encoders().len() - 1);
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

    if !full_bench {
        loop {
            print_options(get_supported_inputs().to_vec());
            print!("\nChoose video file to encode [0-{}]: ", get_supported_inputs().len() - 1);
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

        Vendor::InteliGPU => {
            let intel_qsv = IntelIGPU::new(cli.encoder == "hevc_qsv");
            intel_qsv.get_benchmark_settings()
        }
        Vendor::Unknown => {
            // nothing to do here
            String::from("")
        }
    };
}

fn get_bitrate_for(metadata: &MetaData, string: String) -> u32 {
    if string.contains("nvenc") {
        return *Nvenc::get_resolution_to_bitrate_map(metadata.fps).get(&metadata.get_res()).unwrap();
    } else {
        return *Amf::get_resolution_to_bitrate_map(metadata.fps).get(&metadata.get_res()).unwrap();
    }
}

fn get_input_files(source_file: String) -> Vec<String> {
    if source_file.is_empty() {
        return get_supported_inputs()
            .iter()
            .map(|s| map_file(is_dev(), s))
            .collect::<Vec<String>>();
    }

    return vec![source_file];
}

fn map_file(is_dev: bool, s: &&str) -> String {
    let mut file = String::new();
    if is_dev {
        file.push_str("../");
        file.push_str(*s);
    } else {
        file.push_str(*s);
    }

    return file;
}

fn fig_title(msg: String, small_font_content: String) {
    let small_font_file_name = "tmp.flf";

    // create the font file to use, then delete it
    let mut tmp_font_file = File::create(small_font_file_name).unwrap();
    write!(&mut tmp_font_file, "{}", small_font_content).unwrap();

    let small_font = FIGfont::from_file(small_font_file_name).unwrap();
    let figure = small_font.convert(msg.as_str());
    assert!(figure.is_some());
    println!("{}\n", figure.unwrap());
    println!("Version v0.2.0-alpha");
    println!("Source code: https://github.com/Proryanator/encoder-benchmark\n");

    fs::remove_file(small_font_file_name).expect("Not able to delete tmp file");
}