use clap::Parser;

use codecs::amf::Amf;
use codecs::av1_qsv::AV1QSV;
use codecs::get_vendor_for_codec;
use codecs::nvenc::Nvenc;
use codecs::permute::Permute;
use codecs::qsv::QSV;
use codecs::vendor::Vendor;
use engine::permutation_engine::PermutationEngine;
use permutation::permutation::Permutation;

use crate::permutor_cli::PermutorCli;

mod permutor_cli;

fn main() {
    let mut cli = PermutorCli::parse();
    cli.validate();

    log_special_arguments(&cli);

    let mut engine = PermutationEngine::new();
    let vendor = get_vendor_for_codec(&cli.encoder.clone());
    for bitrate in get_bitrate_permutations(cli.bitrate, cli.max_bitrate_permutation.unwrap()) {
        match vendor {
            Vendor::Nvidia => {
                build_nvenc_setting_permutations(&mut engine, &cli, bitrate);
            }
            Vendor::AMD => {
                build_amf_setting_permutations(&mut engine, &cli, bitrate);
            }
            Vendor::IntelQSV => {
                if cli.encoder.contains("av1") {
                    build_intel_av1_permutations(&mut engine, &cli, bitrate);
                } else {
                    build_intel_igpu_permutations(&mut engine, &cli, bitrate);
                }
            }
            Vendor::Unknown => {}
        }
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

        if cli.allow_duplicate_scores {
            println!("  -ignoring whether expected vmaf score will be duplicated");
        }

        if cli.verbose {
            println!("  -verbose enabled");
        }

        if cli.test_run {
            println!("  -test run, will only run 1 permutation");
        }
    }
}

fn build_nvenc_setting_permutations(engine: &mut PermutationEngine, cli: &PermutorCli, bitrate: u32) {
    let mut nvenc = Nvenc::new(cli.encoder == "hevc_nvenc", cli.gpu);

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
        permutation.allow_duplicates = cli.allow_duplicate_scores;
        permutation.verbose = cli.verbose;
        engine.add(permutation);

        // break out early here to just make 1 permutation
        if cli.test_run {
            break;
        }
    }
}

fn build_amf_setting_permutations(engine: &mut PermutationEngine, cli: &PermutorCli, bitrate: u32) {
    let mut amf = Amf::new(cli.encoder == "hevc_amf", cli.gpu);

    // initialize the permutations each time
    amf.init();

    while let Some((_encoder_index, settings)) = amf.next() {
        let mut permutation = Permutation::new(cli.source_file.clone(), cli.encoder.clone());
        permutation.video_file = cli.source_file.clone();
        permutation.encoder_settings = settings;
        permutation.bitrate = bitrate;
        permutation.check_quality = cli.check_quality;
        permutation.verbose = cli.verbose;
        permutation.detect_overload = cli.detect_overload;
        permutation.allow_duplicates = cli.allow_duplicate_scores;
        engine.add(permutation);

        // break out early here to just make 1 permutation
        if cli.test_run {
            break;
        }
    }
}

fn build_intel_av1_permutations(engine: &mut PermutationEngine, cli: &PermutorCli, bitrate: u32) {
    let mut intel_av1 = AV1QSV::new();

    // initialize the permutations each time
    intel_av1.init();

    while let Some((_encoder_index, settings)) = intel_av1.next() {
        let mut permutation = Permutation::new(cli.source_file.clone(), cli.encoder.clone());
        permutation.video_file = cli.source_file.clone();
        permutation.encoder_settings = settings;
        permutation.bitrate = bitrate;
        permutation.check_quality = cli.check_quality;
        permutation.verbose = cli.verbose;
        permutation.detect_overload = cli.detect_overload;
        permutation.allow_duplicates = cli.allow_duplicate_scores;
        engine.add(permutation);

        // break out early here to just make 1 permutation
        if cli.test_run {
            break;
        }
    }
}

fn build_intel_igpu_permutations(engine: &mut PermutationEngine, cli: &PermutorCli, bitrate: u32) {
    let mut intel_i_gpu = QSV::new(cli.encoder == "hevc_qsv");

    // initialize the permutations each time
    intel_i_gpu.init();

    while let Some((_encoder_index, settings)) = intel_i_gpu.next() {
        let mut permutation = Permutation::new(cli.source_file.clone(), cli.encoder.clone());
        permutation.video_file = cli.source_file.clone();
        permutation.encoder_settings = settings;
        permutation.bitrate = bitrate;
        permutation.check_quality = cli.check_quality;
        permutation.verbose = cli.verbose;
        permutation.detect_overload = cli.detect_overload;
        permutation.allow_duplicates = cli.allow_duplicate_scores;
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