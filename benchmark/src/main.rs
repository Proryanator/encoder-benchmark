use std::{env, fs};
use std::fs::File;
use std::io::Write;

use clap::Parser;
use figlet_rs::FIGfont;
use text_io::read;

use cli::cli_util::{is_dev, pause};
use cli::supported::{get_supported_encoders, get_supported_inputs};
use engine::benchmark_engine::BenchmarkEngine;
use engine::h264_hevc_nvenc::Nvenc;
use engine::permute::Permute;
use ffmpeg::metadata::MetaData;
use permutation::permutation::Permutation;

use crate::benchmark_cli::BenchmarkCli;

mod benchmark_cli;

fn main() {
    let small_font = include_str!("small.flf");

    fig_title(String::from("Encoder-Benchmark"), String::from(small_font));
    let mut cli = BenchmarkCli::new();

    // if no args were provided, they will be prompted from the user
    // this works for both cli running as well as just clicking the executable
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        read_user_input(&mut cli);
        cli.set_ui_opened();
    } else {
        cli = BenchmarkCli::parse();
    }

    cli.validate();

    let input_files = get_input_files(cli.source_file);
    let mut engine = BenchmarkEngine::new();
    let nvenc = Nvenc::new(cli.encoder == "hevc_nvenc");

    // prepare permutations for the engine to run over
    for input in input_files {
        let mut permutation = Permutation::new(input, cli.encoder.clone());
        let settings = get_settings_for(&nvenc);
        let bitrate = get_bitrate_for(&permutation.get_metadata());

        permutation.bitrate = bitrate;
        permutation.encoder_settings = settings;
        engine.add(permutation);
    }

    engine.run();
    pause();
}

fn read_user_input(cli: &mut BenchmarkCli) {
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

fn get_settings_for(nvenc: &Nvenc) -> String {
    // need to support other encoders here
    return nvenc.get_benchmark_settings();
}

fn get_bitrate_for(metadata: &MetaData) -> u32 {
    // need to support other encoders here
    return *Nvenc::get_resolution_to_bitrate_map(metadata.fps).get(&metadata.get_res()).unwrap();
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