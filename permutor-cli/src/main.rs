use clap::Parser;

use engine::h264_hevc_nvenc::Nvenc;
use engine::permutation_engine::PermutationEngine;
use engine::permute::Permute;
use permutation::permutation::Permutation;

use crate::permutor_cli::PermutorCli;

mod permutor_cli;

fn main() {
    let mut cli = PermutorCli::parse();
    cli.validate();

    log_special_arguments(&cli);

    let mut engine = PermutationEngine::new();
    let mut nvenc = Nvenc::new(cli.encoder == "hevc_nvenc", cli.gpu);
    for bitrate in get_bitrate_permutations(cli.bitrate, cli.max_bitrate_permutation.unwrap()) {
        build_setting_permutations(&mut engine, &mut nvenc, &cli, bitrate);
    }

    engine.run();
}

fn log_special_arguments(cli: &PermutorCli) {
    if cli.has_special_options() {
        println!("\nOptions:");
        if cli.detect_overload {
            println!("  -encoding will stop if overload detected");
        }

        if cli.check_quality {
            println!("  -calculating vmaf score");
        }

        if cli.verbose {
            println!("  -verbose enabled");
        }

        if cli.test_run {
            println!("  -test run, will only run 1 permutation");
        }
    }
}

fn build_setting_permutations(engine: &mut PermutationEngine, nvenc: &mut Nvenc, cli: &PermutorCli, bitrate: u32) {
    // initialize the permutations each time
    nvenc.init();

    while let Some((_encoder_index, settings)) = nvenc.next() {
        let mut permutation = Permutation::new(cli.source_file.clone(), cli.encoder.clone());
        permutation.video_file = cli.source_file.clone();
        permutation.encoder_settings = settings;
        permutation.bitrate = bitrate;
        permutation.check_quality = cli.check_quality;
        permutation.verbose = cli.verbose;
        permutation.detect_overload = cli.detect_overload;
        engine.add(permutation);

        // break out early here to just make 1 permutation
        if cli.test_run {
            break;
        }
    }
}

fn get_bitrate_permutations(starting_bitrate: u32, max_bitrate: u32) -> Vec<u32> {
    let interval = 5;
    let mut bitrates = Vec::new();
    for i in 0..(((max_bitrate - starting_bitrate) / interval) + 1) {
        bitrates.push(starting_bitrate + (interval * i));
    }

    return bitrates;
}