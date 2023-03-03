use std::collections::HashSet;
use std::ffi::c_float;
use std::fs;
use std::time::{Duration, SystemTime};

use compound_duration::format_dhms;
use crossbeam_channel::Receiver;
use ctrlc::Error;

use ffmpeg::args::FfmpegArgs;
use ffmpeg::report_files::{extract_vmaf_score, get_latest_ffmpeg_report_file, read_last_line_at};
use permutation::permutation::Permutation;

use crate::engine::{log_permutation_header, run_encode, spawn_ffmpeg_child};
use crate::progressbar;
use crate::progressbar::draw_yellow_bar;
use crate::result::{log_results_to_file, PermutationResult};
use crate::threads::setup_ctrl_channel;

pub static TCP_OUTPUT: &str = "-f {} tcp://127.0.0.1:2000";

// the hard-coded vmaf quality we want to shoot for when doing bitrate permutations
const TARGET_QUALITY: c_float = 95.0;

pub struct PermutationEngine {
    permutations: Vec<Permutation>,
    results: Vec<PermutationResult>,
    dup_results: Vec<PermutationResult>,
    vmaf_scores: HashSet<String>,
}

// note: we can make 2 engines; benchmark engine, and the permutation engine
// this way we can make the run() method a lot less complex
impl PermutationEngine {
    pub fn new() -> Self {
        return Self {
            permutations: vec![],
            results: vec![],
            dup_results: vec![],
            vmaf_scores: HashSet::new(),
        };
    }

    pub fn run(&mut self) {
        let runtime = SystemTime::now();
        let ctrl_channel = setup_ctrl_channel();
        let mut target_quality_found = false;

        let mut ignore_factor = 1 as c_float;
        let mut calc_time: Option<Duration> = None;
        for i in 0..self.permutations.clone().len() {
            let permutation_start_time = SystemTime::now();
            let mut permutation = self.permutations[i].clone();
            log_permutation_header(i, &self.permutations, calc_time, ignore_factor);

            // if this permutation was added to the list of duplicates, skip to save calculation time
            if !permutation.allow_duplicates && permutation.check_quality && will_be_duplicate(&self.dup_results, &permutation) {
                draw_yellow_bar(permutation.get_metadata().frames);
                println!("\n!!! Above encoder settings will produce identical vmaf score as other permutations, skipping... \n");
                continue;
            }

            let mut result = run_encode(permutation.clone(), &ctrl_channel);
            calc_time = Option::from(permutation_start_time.elapsed().unwrap());

            if !result.was_overloaded && permutation.check_quality.clone() {
                let vmaf_start_time = SystemTime::now();
                result.vmaf_score = check_encode_quality(permutation.clone(), &ctrl_channel);
                result.vmaf_calculation_time = vmaf_start_time.elapsed().unwrap().as_secs();

                // if this is higher than the target quality, stop at this bitrate during benchmark
                if result.vmaf_score >= TARGET_QUALITY {
                    target_quality_found = true;
                }

                // take the vmaf calculation time into account for the total ETA calculation
                calc_time = Option::from(permutation_start_time.elapsed().unwrap());
            }

            let is_initial_bitrate_permutation_over = i == self.permutations.len() - 1 || self.permutations[i + 1].clone().bitrate != permutation.bitrate;
            self.add_result(result, is_initial_bitrate_permutation_over, permutation.check_quality, permutation.allow_duplicates);

            // we'll calculate the ignore factor of permutations that will be skipped
            if is_initial_bitrate_permutation_over {
                // % of permutations that we will actually permute over past the initial bitrate
                let perm_count = self.results.len() + self.dup_results.len();
                ignore_factor = self.results.len() as c_float / perm_count as c_float;
            }

            // stop if we've found the target quality, and we're done permuting over the current bitrate
            if target_quality_found && is_initial_bitrate_permutation_over {
                println!("Found VMAF score >= {}, stopping permutations...", TARGET_QUALITY);
                break;
            }
        }

        // produce output files and other logging here
        let runtime_str = format_dhms(runtime.elapsed().unwrap().as_secs());
        log_results_to_file(self.results.clone(), &runtime_str, self.dup_results.clone(), self.permutations[0].bitrate, false);
        println!("Benchmark runtime: {}", runtime_str);
    }

    pub fn add(&mut self, permutation: Permutation) {
        self.permutations.push(permutation);
    }

    fn add_result(&mut self, result: PermutationResult, is_bitrate_permutation_over: bool, is_checking_quality: bool, allow_duplicates: bool) {
        // only do this duplicate mapping during the first bitrate permutation
        // notice how we do not add this for overloaded results
        if !allow_duplicates && is_checking_quality && !result.was_overloaded && !is_bitrate_permutation_over {
            let score_str = result.vmaf_score.to_string();
            if !self.vmaf_scores.contains(&score_str) {
                self.results.push(result);
                self.vmaf_scores.insert((*score_str).parse().unwrap());
            } else {
                self.dup_results.push(result);
            }
        } else {
            // always keep results; any duplicates will be ignored already
            self.results.push(result);
        }
    }
}

fn check_encode_quality(mut p: Permutation, ctrl_channel: &Result<Receiver<()>, Error>) -> c_float {
    let ffmpeg_args = FfmpegArgs::build_ffmpeg_args(p.video_file.clone(), p.encoder.clone(), &p.encoder_settings, p.bitrate.clone());

    println!("Calculating vmaf score; might take longer than original encode depending on your CPU...");

    let metadata = p.get_metadata();
    // first spawn the ffmpeg instance to listen for incoming encode
    let vmaf_args = ffmpeg_args.map_to_vmaf(metadata.fps);
    if p.verbose {
        println!("Vmaf args calculating quality: {}", vmaf_args.to_string());
    }

    let mut vmaf_child = spawn_ffmpeg_child(&vmaf_args);

    // then spawn the ffmpeg instance to perform the encoding
    let mut encoder_args = ffmpeg_args.clone();

    encoder_args.output_args = String::from(insert_format_from(TCP_OUTPUT, &ffmpeg_args.encoder));

    if p.verbose {
        println!("Encoder fmmpeg args sending to vmaf: {}", encoder_args.to_string());
    }

    spawn_ffmpeg_child(&encoder_args);

    // not the cleanest way to do this but oh well
    progressbar::watch_encode_progress(metadata.frames, false, metadata.fps, false, ffmpeg_args.stats_period, ctrl_channel);

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

fn will_be_duplicate(duplicates: &Vec<PermutationResult>, next_permutation: &Permutation) -> bool {
    for dup in duplicates {
        if dup.encoder_settings == next_permutation.encoder_settings {
            return true;
        }
    }

    return false;
}

fn insert_format_from(input: &str, encoder: &String) -> String {
    let format = if encoder == "h264_nvenc" { "h264" } else { "hevc" };
    // this should be cleaner when we support more than 1 type
    return input.replace("{}", format);
}