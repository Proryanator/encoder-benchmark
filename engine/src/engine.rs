use std::ffi::c_float;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use compound_duration::format_dhms;
use crossbeam_channel::Receiver;
use ctrlc::Error;

use ffmpeg::args::FfmpegArgs;
use ffmpeg::metadata::MetaData;
use permutation::permutation::Permutation;

use crate::progressbar;
use crate::progressbar::TrialResult;
use crate::result::PermutationResult;
use crate::threads::was_ctrl_c_received;

pub fn run_encode(mut p: Permutation, ctrl_channel: &Result<Receiver<()>, Error>) -> PermutationResult {
    let mut result = PermutationResult::new(&p.get_metadata(), p.bitrate, &p.encoder_settings, &p.encoder);

    let metadata = p.get_metadata();

    let ffmpeg_args = FfmpegArgs::build_ffmpeg_args(p.video_file, p.encoder, &p.encoder_settings, p.bitrate);

    let encode_start_time = SystemTime::now();

    // not sure what to do about these results here
    let mut trial_result = run_overload_benchmark(&metadata, &ffmpeg_args, p.verbose, p.detect_overload, &ctrl_channel);

    result.was_overloaded = trial_result.was_overloaded;
    result.encode_time = encode_start_time.elapsed().unwrap().as_secs();

    // calculate the fps statistics and store this in the result
    calculate_fps_statistics(&mut result, &mut trial_result);

    // log the calculated fps statistics; two spaces match the progress bar
    println!("  Average FPS:\t{:.0}", result.fps_stats.avg);
    println!("  1%'ile:\t{}", result.fps_stats.one_perc_low);
    println!("  90%'ile:\t{}\n", result.fps_stats.ninety_perc);

    return result;
}

pub fn log_permutation_header(index: usize, permutations: &Vec<Permutation>, calc_time: Option<Duration>, ignore_factor: c_float) {
    log_header(index, permutations, calc_time, true, ignore_factor);
}

pub fn log_benchmark_header(index: usize, permutations: &Vec<Permutation>, calc_time: Option<Duration>) {
    log_header(index, permutations, calc_time, false, 1 as c_float);
}

fn log_header(index: usize, permutations: &Vec<Permutation>, calc_time: Option<Duration>, log_eta: bool, ignore_factor: c_float) {
    let mut permutation = permutations[index].clone();
    let metadata = permutation.get_metadata();
    if log_eta {
        if calc_time.is_some() {
            println!("[ETR: {}]", format_dhms(calculate_eta(calc_time.unwrap(), index, permutations.len(), ignore_factor)));
        } else {
            println!("[ETR: Unknown until first permutation is done]");
        }
    }
    println!("[Permutation:\t{}/{}]", index + 1, permutations.len());
    println!("[Resolution:\t{}x{}]", metadata.width, metadata.height);
    println!("[Encoder:\t{}]", permutation.encoder);
    println!("[FPS:\t\t{}]", metadata.fps);
    println!("[Bitrate:\t{}Mb/s]", permutation.bitrate);
    println!("[{}]", permutation.encoder_settings);
}

pub fn spawn_ffmpeg_child(ffmpeg_args: &FfmpegArgs) -> Child {
    return Command::new("ffmpeg")
        .args(ffmpeg_args.to_vec())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn().expect("Failed to start instance of ffmpeg");
}

fn run_overload_benchmark(metadata: &MetaData, ffmpeg_args: &FfmpegArgs, verbose: bool, detect_overload: bool, ctrl_channel: &Result<Receiver<()>, Error>) -> TrialResult {
    let mut child = spawn_ffmpeg_child(ffmpeg_args);
    if verbose {
        println!("Successfully spawned encoding child")
    }

    let trial_result = progressbar::watch_encode_progress(metadata.frames, detect_overload, metadata.fps, verbose, ffmpeg_args.stats_period, ctrl_channel);

    if trial_result.was_overloaded && !was_ctrl_c_received(&ctrl_channel) {
        let _ = child.kill();
        println!("Encoder was overloaded and could not encode the video file in realtime, stopping...");
    }

    return trial_result;
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

fn calculate_eta(elapsed: Duration, current_perm: usize, total_perms: usize, ignored_factor: c_float) -> usize {
    let seconds = elapsed.as_secs() as usize;
    let remaining_permutations = total_perms - (current_perm - 1);
    return (((seconds * remaining_permutations) as c_float) * ignored_factor) as usize;
}