use std::collections::HashSet;
use std::ffi::c_float;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use clap::Parser;
use compound_duration::format_dhms;
use crossbeam_channel::{bounded, Receiver, select};
use ctrlc::Error;

use ffmpeg::metadata::MetaData;

use crate::cli::Cli;
use crate::encode_file_downloader::are_all_source_files_present;
use crate::env::fail_if_environment_not_setup;
use crate::ffmpeg::args::FfmpegArgs;
use crate::ffmpeg::ffprobe::probe_for_video_metadata;
use crate::ffmpeg::report_files::{extract_vmaf_score, get_latest_ffmpeg_report_file, read_last_line_at};
use crate::permutations::h264_hevc_nvenc::Nvenc;
use crate::permutations::permute::Permute;
use crate::permutations::result::{log_results_to_file, PermutationResult};
use crate::progressbar::{draw_yellow_bar, TrialResult};

mod progressbar;
mod permutations;
mod ffmpeg;
mod cli;
mod stat_tcp_listener;
mod env;
mod encode_file_downloader;

// the hard-coded vmaf quality we want to shoot for when doing bitrate permutations
const TARGET_QUALITY: c_float = 95.0;

pub static TCP_OUTPUT: &str = "-f {} tcp://127.0.0.1:2000";

// list of all encode files
const ENCODE_FILES: [&'static str; 8] = ["720-60.y4m", "720-120.y4m", "1080-60.y4m", "1080-120.y4m", "2k-60.y4m", "2k-120.y4m", "4k-60.y4m", "4k-120.y4m"];

const DOWNLOAD_URL: &str = "https://www.dropbox.com/sh/x08pkk47lc1v5ex/AADGaoOjOcA0-uPo7I0NaxL-a?dl=0";

fn main() {
    let runtime = SystemTime::now();

    fail_if_environment_not_setup();
    let mut cli = Cli::parse();
    cli.validate();

    let ctrl_channel = setup_ctrl_channel();

    let is_standard_benchmark = cli.source_file.is_empty();

    if is_standard_benchmark && !are_all_source_files_present() {
        println!("You're missing some video source files to run the standard benchmark; you should have the following: \n{:?}", ENCODE_FILES);
        println!("Please download the ones you are missing from: {}", DOWNLOAD_URL);
        println!("If you want to run the tool against a specific resolution/fps, download just that source file and specify it with '-s'");
        std::process::exit(1);
    }

    // eventually we'll want to support more than just these two
    let mut permutation = Nvenc::new(cli.encoder == "hevc_nvenc");
    let encoder_settings = if is_standard_benchmark { permutation.run_standard_only() } else { permutation.init() };

    // determining if we'll be iterating over X number of input files, or just the provided one
    let mut source_files = <Vec<&str>>::new();
    let specified_source_file = (&cli.source_file).clone();
    if is_standard_benchmark {
        for source in ENCODE_FILES {
            source_files.push(source);
        }

        // set initial source file
        cli.source_file = source_files.get(0).unwrap().to_string();
    } else {
        source_files.push(specified_source_file.as_str());
    };

    let total_encoder_permutations = encoder_settings.len();
    let bitrate_permutations = get_bitrate_permutations(cli.bitrate, cli.max_bitrate_permutation.unwrap());
    let total_permutations = if cli.test_run { 1 } else { encoder_settings.len() * bitrate_permutations.len() * source_files.len() };

    let mut all_results: Vec<PermutationResult> = Vec::new();
    let mut dup_results: Vec<PermutationResult> = Vec::new();
    let mut vmaf_scores: HashSet<String> = HashSet::new();

    let mut permutation_calculation_time: Option<Duration> = None;

    let mut target_quality_found = false;

    // loop over all permutations of bitrate we want to try
    let mut bitrate_index = 0;

    // factor to apply to the ETA for whether we're ignoring any permutations
    let mut ignore_factor: c_float = 1f32;

    for input in source_files {
        // apply the input to the cli args
        cli.source_file = input.to_string();

        let metadata = probe_for_video_metadata(&cli.source_file);

        // we'll calculate this as if this is the standard benchmark
        let res_to_bitrate_map = permutation.get_resolution_to_bitrate_map(metadata.fps);

        // do not check for encoder overload
        let ignore_overload = !cli.detect_overload;

        if cli.verbose {
            println!("{}", metadata.to_string());
        }

        // this is the actual bitrate we'll use
        let mut effective_bitrate = if is_standard_benchmark { *(res_to_bitrate_map.get(metadata.get_res().as_str()).unwrap()) } else { cli.bitrate };

        log_total_permutations(&cli, &metadata, total_permutations, bitrate_permutations.len(), is_standard_benchmark, effective_bitrate);

        for bitrate in bitrate_permutations.clone() {
            if !is_standard_benchmark {
                effective_bitrate = bitrate;
            }

            if target_quality_found && !is_standard_benchmark {
                println!("Found vmaf score >= {}, stopping benchmark...", TARGET_QUALITY);
                break;
            }

            // for a given bitrate, permute over all possible encoder settings
            while let Some((encoder_index, settings)) = permutation.next() {

                // add in loop over all supported input files
                let mut result = PermutationResult::new(&metadata, effective_bitrate, &settings, cli.encoder.as_str());

                let ffmpeg_args = FfmpegArgs::build_ffmpeg_args(&cli, &settings, effective_bitrate);
                if cli.verbose {
                    println!("ffmpeg args: {}", ffmpeg_args.to_string());
                }

                let permutation_start_time = SystemTime::now();

                let current_index = (bitrate_index * total_encoder_permutations) + (encoder_index + 1);
                log_permutation_header(total_permutations, permutation_calculation_time, effective_bitrate, current_index, settings, ignore_factor, is_standard_benchmark);

                // if this permutation was added to the list of duplicates, skip to save calculation time
                if !cli.allow_duplicate_scores && will_be_duplicate(&dup_results, &result) {
                    draw_yellow_bar(metadata.frames);
                    println!("\n!!! Above encoder settings will produce identical vmaf score as other permutations, skipping... \n");
                    continue;
                }

                let encode_start_time = SystemTime::now();
                let mut trial_result = run_overload_benchmark(&metadata, &ffmpeg_args, cli.verbose, ignore_overload, &ctrl_channel);
                result.was_overloaded = trial_result.was_overloaded;
                result.encode_time = encode_start_time.elapsed().unwrap().as_secs();

                // calculate the fps statistics and store this in the result
                calculate_fps_statistics(&mut result, &mut trial_result);

                // log the calculated fps statistics; two spaces match the progress bar
                println!("  Average FPS:\t{:.0}", result.fps_stats.avg);
                println!("  1%'ile:\t{}", result.fps_stats.one_perc_low);
                println!("  90%'ile:\t{}", result.fps_stats.ninety_perc);

                // retry once if it was overloaded, but only if we're not ignoring overloads
                if !ignore_overload && result.was_overloaded {
                    println!("Retrying encode just in case this overload was a one-off...");
                    let encode_start_time = SystemTime::now();
                    let mut trial_result = run_overload_benchmark(&metadata, &ffmpeg_args, cli.verbose, ignore_overload, &ctrl_channel);
                    result.was_overloaded = trial_result.was_overloaded;
                    result.encode_time = encode_start_time.elapsed().unwrap().as_secs();

                    // calculate the fps statistics and store this in the result
                    calculate_fps_statistics(&mut result, &mut trial_result);

                    // log the calculated fps statistics; two spaces match the progress bar
                    println!("  Average FPS:\t{:.0}", result.fps_stats.avg);
                    println!("  1%'ile:\t{}", result.fps_stats.one_perc_low);
                    println!("  90%'ile:\t{}", result.fps_stats.ninety_perc);
                }

                // set the permutation calculation time; if we're doing vmaf score, will update a second time
                permutation_calculation_time = Option::from(permutation_start_time.elapsed().unwrap());

                // if we're not calculating vmaf score, continue to the next permutation
                if cli.check_quality {
                    // if it's still overloaded, we won't be able to check this
                    if !result.was_overloaded {
                        let vmaf_start_time = SystemTime::now();
                        result.vmaf_score = check_encode_quality(&metadata, &ffmpeg_args, cli.verbose, &ctrl_channel);
                        result.vmaf_calculation_time = vmaf_start_time.elapsed().unwrap().as_secs();

                        // if this is higher than the target quality, stop at this bitrate during benchmark
                        if result.vmaf_score >= TARGET_QUALITY {
                            target_quality_found = true;
                        }

                        // update each permutation count
                        permutation_calculation_time = Option::from(permutation_start_time.elapsed().unwrap());
                    } else {
                        println!("Will not check vmaf score as encoder cannot handle realtime encoding of given parameters...\n");
                    }

                    // only do this duplicate mapping during the first bitrate permutation
                    // notice how we do not add this for overloaded results
                    if !result.was_overloaded && bitrate_index == 0 {
                        let score_str = result.vmaf_score.to_string();
                        if !vmaf_scores.contains(&score_str) {
                            all_results.push(result);
                            vmaf_scores.insert((*score_str).parse().unwrap());
                        } else {
                            dup_results.push(result);
                        }
                    } else {
                        // always keep results; any duplicates will be ignored already
                        all_results.push(result);
                    }
                } else {
                    all_results.push(result);
                }

                if cli.test_run {
                    break;
                }
            }

            if cli.test_run {
                break;
            }

            // if all permutations resulted in an overload, skip additional bitrate permutations
            if did_all_permutions_fail(&all_results) {
                println!("None of the permutations on the starting bitrate were successful; stopping benchmark...");
                break;
            }

            // only log this after the first bitrate permutation
            if cli.check_quality && !cli.allow_duplicate_scores && bitrate_index == 0 {
                println!("!!! Permutations past this point will only run if they are expected to have unique vmaf scores !!!");

                // number of actually calculated permutations out of the original total
                ignore_factor = all_results.len() as c_float / total_encoder_permutations as c_float;
            }

            // re-init the encoder settings to run with the next bitrate
            if is_standard_benchmark {
                permutation.run_standard_only();
            } else {
                permutation.init();
            }
            bitrate_index += 1;
        }
    }

    let runtime_str = format_dhms(runtime.elapsed().unwrap().as_secs());
    log_results_to_file(all_results, &runtime_str, dup_results, cli.bitrate, is_standard_benchmark);

    println!("Benchmark runtime: {}", runtime_str);
}

fn did_all_permutions_fail(results: &Vec<PermutationResult>) -> bool {
    return results.into_iter().all(|x| x.was_overloaded == true);
}

fn calculate_fps_statistics(permutation_result: &mut PermutationResult, trial_result: &mut TrialResult) {
    // must use a much larger data type for calculating the average
    let mut sum: u64 = 0;
    for fps in &trial_result.all_fps {
        sum += *fps as u64;
    }

    permutation_result.fps_stats.avg = (sum as usize / trial_result.all_fps.len()) as u16;

    // create a sorted list of the fps measurements
    trial_result.all_fps.sort();

    // find the index & calculate 1%ile
    let mut index = (0.01 as c_float * trial_result.all_fps.len() as c_float).ceil();
    permutation_result.fps_stats.one_perc_low = *(trial_result.all_fps.get(index as usize).unwrap());

    // find the index & calculate 90%ile
    index = (0.90 as c_float * trial_result.all_fps.len() as c_float).ceil();
    permutation_result.fps_stats.ninety_perc = *(trial_result.all_fps.get(index as usize).unwrap());
}

fn will_be_duplicate(duplicates: &Vec<PermutationResult>, result: &PermutationResult) -> bool {
    for dup in duplicates {
        if dup.encoder_settings == result.encoder_settings {
            return true;
        }
    }

    return false;
}

fn log_permutation_header(permutation_count: usize, permutation_calculation_time: Option<Duration>, bitrate: u32, index: usize, settings: String, ignored_factor: c_float, is_standard: bool) {
    if index != 1 && !is_standard {
        println!("====================================================================================");
    }

    if !is_standard {
        println!("[Permutation {}/{}]", index, permutation_count);
    }

    if !is_standard {
        if permutation_calculation_time.is_some() {
            println!("[ETR: {}]", format_dhms(calculate_eta(permutation_calculation_time.unwrap(), index, permutation_count, ignored_factor)));
        } else {
            println!("[ETR: Unknown until first permutation is done]");
        }
    }

    println!("[Bitrate: {}Mb/s]", bitrate);
    println!("[{}]", settings);
}

fn calculate_eta(elapsed: Duration, current_perm: usize, total_perms: usize, ignored_factor: c_float) -> usize {
    let seconds = elapsed.as_secs() as usize;
    let remaining_permutations = total_perms - (current_perm - 1);
    return (((seconds * remaining_permutations) as c_float) * ignored_factor) as usize;
}

fn log_total_permutations(cli: &Cli, metadata: &MetaData, permutation_count: usize, bitrate_permutations: usize, is_standard: bool, effective_bitrate: u32) {
    println!("====================================================================================");
    if !is_standard {
        println!("Permutations:\t{}", permutation_count);
    }
    println!("Resolution:\t{}x{}", metadata.width, metadata.height);
    println!("Encoder:\t{}", cli.encoder);
    if bitrate_permutations > 1 {
        println!("Min bitrate:\t{}Mb/s", cli.bitrate);
        println!("Max bitrate:\t{}Mb/s", cli.max_bitrate_permutation.unwrap());
    } else {
        println!("Bitrate:\t{}Mb/s", effective_bitrate);
    }

    println!("FPS:\t\t{}", metadata.fps);

    // might move this to print somewhere earlier and not here
    if cli.has_special_options() && !is_standard {
        println!("\nOptions:");
        if cli.detect_overload {
            println!("  -encoder will stop if overload is detected");
        }

        if cli.check_quality {
            println!("  -calculating vmaf score");
        }

        if cli.allow_duplicate_scores {
            println!("  -allowing duplicate vmaf scores");
        }
    }

    println!();
}

fn get_bitrate_permutations(starting_bitrate: u32, max_bitrate: u32) -> Vec<u32> {
    let interval = 5;
    let mut bitrates = Vec::new();
    for i in 0..(((max_bitrate - starting_bitrate) / interval) + 1) {
        bitrates.push(starting_bitrate + (interval * i));
    }

    return bitrates;
}

fn check_encode_quality(metadata: &MetaData, ffmpeg_args: &FfmpegArgs, verbose: bool, ctrl_channel: &Result<Receiver<()>, Error>) -> c_float {
    println!("Calculating vmaf score; might take longer than original encode depending on your CPU...");

    // first spawn the ffmpeg instance to listen for incoming encode
    let vmaf_args = ffmpeg_args.map_to_vmaf(metadata.fps);
    if verbose {
        println!("Vmaf args calculating quality: {}", vmaf_args.to_string());
    }

    let mut vmaf_child = spawn_ffmpeg_child(&vmaf_args);

    // then spawn the ffmpeg instance to perform the encoding
    let mut encoder_args = ffmpeg_args.clone();

    encoder_args.output_args = String::from(insert_format_from(TCP_OUTPUT, &ffmpeg_args.encoder));

    if verbose {
        println!("Encoder fmmpeg args sending to vmaf: {}", encoder_args.to_string());
    }

    spawn_ffmpeg_child(&encoder_args);

    // not the cleanest way to do this but oh well
    progressbar::watch_encode_progress(metadata.frames, false, metadata.fps, verbose, ffmpeg_args.stats_period, ctrl_channel);

    // need to wait for the vmaf calculating thread to finish
    println!("VMAF calculation finishing up...");
    vmaf_child.wait().expect("Not able to wait on the child thread to finish up");

    let vmaf_log_file = get_latest_ffmpeg_report_file();
    let vmaf_score_extract = extract_vmaf_score(read_last_line_at(3).as_str());
    let vmaf_score = vmaf_score_extract.unwrap();
    println!("VMAF score: {}\n", vmaf_score);

    // cleanup the log file being used
    fs::remove_file(vmaf_log_file.as_path()).unwrap();

    return vmaf_score;
}

fn insert_format_from(input: &str, encoder: &String) -> String {
    let format = if encoder == "h264_nvenc" { "h264" } else { "hevc" };
    // this should be cleaner when we support more than 1 type
    return input.replace("{}", format);
}

fn run_overload_benchmark(metadata: &MetaData, ffmpeg_args: &FfmpegArgs, verbose: bool, ignore_overload: bool, ctrl_channel: &Result<Receiver<()>, Error>) -> TrialResult {
    let mut child = spawn_ffmpeg_child(ffmpeg_args);
    if verbose {
        println!("Successfully spawned encoding child")
    }

    let trial_result = progressbar::watch_encode_progress(metadata.frames, !ignore_overload, metadata.fps, verbose, ffmpeg_args.stats_period, ctrl_channel);

    if trial_result.was_overloaded && !was_ctrl_c_received(&ctrl_channel) {
        let _ = child.kill();
        println!("Encoder was overloaded and could not encode the video file in realtime, stopping...");
    }

    return trial_result;
}

fn spawn_ffmpeg_child(ffmpeg_args: &FfmpegArgs) -> Child {
    return Command::new("ffmpeg")
        .args(ffmpeg_args.to_vec())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn().expect("Failed to start instance of ffmpeg");
}

// probably want to make this more robust where it kills all child threads instead of waiting
fn setup_ctrl_channel() -> Result<Receiver<()>, Error> {
    let (sender, receiver) = bounded(100);
    ctrlc::set_handler(move || {
        println!("Received ctrl-c, exiting gracefully...");
        let _ = sender.send(());
    })?;

    Ok(receiver)
}

fn was_ctrl_c_received(ctrl_c_events: &Result<Receiver<()>, Error>) -> bool {
    select! {
            recv(ctrl_c_events.as_ref().unwrap()) -> _ => {
                return true;
            },
            default() => {
                return false;
            }
        }
}

fn exit_on_ctrl_c(ctrl_channel: &Result<Receiver<()>, Error>) {
    if was_ctrl_c_received(&ctrl_channel) {
        println!("Ctrl-C acknowledged, program exiting...");
        std::process::exit(0);
    }
}
