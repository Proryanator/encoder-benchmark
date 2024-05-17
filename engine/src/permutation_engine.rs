use std::collections::HashSet;
use std::ffi::c_float;
use std::fs::{remove_file, rename};
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

pub static TCP_OUTPUT: &str = "-f {} tcp://localhost:2000";

// the hard-coded vmaf quality we want to shoot for when doing bitrate permutations
const TARGET_QUALITY: c_float = 95.0;

// the hard-coded value for max retries to calculate vmaf score
const MAX_ATTEMPTS_CALC_QUALITY: i32 = 3;

pub struct PermutationEngine {
    permutations: Vec<Permutation>,
    results: Vec<PermutationResult>,
    dup_results: Vec<PermutationResult>,
    vmaf_scores: HashSet<String>,
    log_files_directory: String,
}

// note: we can make 2 engines; benchmark engine, and the permutation engine
// this way we can make the run() method a lot less complex
impl PermutationEngine {
    pub fn new(log_files: String) -> Self {
        return Self {
            permutations: vec![],
            results: vec![],
            dup_results: vec![],
            vmaf_scores: HashSet::new(),
            log_files_directory: log_files,
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
            if !permutation.allow_duplicates
                && permutation.check_quality
                && will_be_duplicate(&self.dup_results, &permutation)
            {
                draw_yellow_bar(permutation.get_metadata().frames);
                println!("\n!!! Above encoder settings will produce identical vmaf score as other permutations, skipping... \n");
                continue;
            }

            let mut result = run_encode(permutation.clone(), &ctrl_channel);
            calc_time = Option::from(permutation_start_time.elapsed().unwrap());

            if !result.was_overloaded && permutation.check_quality.clone() {
                let vmaf_start_time = SystemTime::now();
                result.vmaf_score = check_encode_quality(
                    &mut permutation.clone(),
                    &ctrl_channel,
                    permutation.verbose,
                    i,
                )
                .expect("Failed to check encode quality");

                result.vmaf_calculation_time = vmaf_start_time.elapsed().unwrap().as_secs();

                // if this is higher than the target quality, stop at this bitrate during benchmark
                if result.vmaf_score >= TARGET_QUALITY {
                    target_quality_found = true;
                }

                // take the vmaf calculation time into account for the total ETA calculation
                calc_time = Option::from(permutation_start_time.elapsed().unwrap());
            }

            let is_initial_bitrate_permutation_over = i == self.permutations.len() - 1
                || self.permutations[i + 1].clone().bitrate != permutation.bitrate;
            self.add_result(
                result,
                is_initial_bitrate_permutation_over,
                permutation.check_quality,
                permutation.allow_duplicates,
            );

            // we'll calculate the ignore factor of permutations that will be skipped
            if is_initial_bitrate_permutation_over {
                // % of permutations that we will actually permute over past the initial bitrate
                let perm_count = self.results.len() + self.dup_results.len();
                ignore_factor = self.results.len() as c_float / perm_count as c_float;
            }

            // stop if we've found the target quality, and we're done permuting over the current bitrate
            if target_quality_found && is_initial_bitrate_permutation_over {
                println!(
                    "Found VMAF score >= {}, stopping permutations...",
                    TARGET_QUALITY
                );
                break;
            }
        }

        // produce output files and other logging here
        let runtime_str = format_dhms(runtime.elapsed().unwrap().as_secs());

        log_results_to_file(
            self.results.clone(),
            &runtime_str,
            self.dup_results.clone(),
            self.permutations[0].bitrate,
            false,
            &self.log_files_directory,
        );
        println!("Benchmark runtime: {}", runtime_str);
    }

    pub fn add(&mut self, permutation: Permutation) {
        self.permutations.push(permutation);
    }

    fn add_result(
        &mut self,
        result: PermutationResult,
        is_bitrate_permutation_over: bool,
        is_checking_quality: bool,
        allow_duplicates: bool,
    ) {
        // only do this duplicate mapping during the first bitrate permutation
        // notice how we do not add this for overloaded results
        if !allow_duplicates
            && is_checking_quality
            && !result.was_overloaded
            && !is_bitrate_permutation_over
        {
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

fn calc_vmaf_score(
    p: &mut Permutation,
    ctrl_channel: &Result<Receiver<()>, Error>,
    verbose: bool,
    attempt: i32,
    perm_num: usize,
) -> Option<c_float> {
    let ffmpeg_args = FfmpegArgs::build_ffmpeg_args(
        p.video_file.clone(),
        p.encoder.clone(),
        &p.encoder_settings,
        p.bitrate.clone(),
        p.decode_run,
    );

    println!(
        "Calculating vmaf score; might take longer than original encode depending on your CPU..."
    );

    let metadata = p.get_metadata();
    // first spawn the ffmpeg instance to listen for incoming encode
    let vmaf_args = ffmpeg_args.map_to_vmaf(metadata.fps);
    if verbose {
        println!(
            "V: Vmaf args calculating quality: {}",
            vmaf_args.to_string()
        );
    }

    let mut vmaf_child = spawn_ffmpeg_child(&vmaf_args, verbose, None);

    // then spawn the ffmpeg instance to perform the encoding
    let mut encoder_args = ffmpeg_args.clone();

    encoder_args.output_args = String::from(insert_format_from(TCP_OUTPUT, &ffmpeg_args.encoder));

    if verbose {
        println!(
            "V: Encoder fmmpeg args sending to vmaf: {}",
            encoder_args.to_string()
        );
    }

    let mut encoder_child = spawn_ffmpeg_child(&encoder_args, verbose, None);

    // not the cleanest way to do this but oh well
    progressbar::watch_encode_progress(
        metadata.frames,
        false,
        metadata.fps,
        false,
        ffmpeg_args.stats_period,
        ctrl_channel,
    );

    // need to wait for the vmaf calculating thread to finish
    println!("VMAF calculation finishing up...");
    let vmaf_child_status = vmaf_child.wait().expect("Vmaf child could not wait");
    let vmaf_log_file = get_latest_ffmpeg_report_file();
    // TODO: this does fix the issue for apple however, this may not scale very well across other vendors
    // or the output line number may have changed recently where we'll need to make this not dependent on line numbers at all
    let line_number = if encoder_args.encoder.contains("videotoolbox") {15} else {3};
    let vmaf_score_line = read_last_line_at(line_number);
    //Cleanup process
    encoder_child
        .kill()
        .expect("Could not kill encoder process");

    if vmaf_child_status.success() {
        let vmaf_score_extract = extract_vmaf_score(vmaf_score_line.as_str());
        let vmaf_score = vmaf_score_extract.expect(&format!(
            "Could not parse score from line: {}",
            vmaf_score_line
        ));
        println!("VMAF score: {}\n", vmaf_score);
        // Cleanup log file
        remove_file(vmaf_log_file.as_path()).unwrap();
        return Some(vmaf_score);
    }

    let org_filename = vmaf_log_file.file_name().unwrap().to_str().unwrap();
    let mut ffmpeg_error_log = vmaf_log_file.clone();
    ffmpeg_error_log.set_extension("");
    let ffmpeg_error_log_filename = ffmpeg_error_log.file_name().unwrap().to_str().unwrap();
    let new_filename = format!(
        "{}-perm-{}-attempt-{}.log",
        ffmpeg_error_log_filename,
        perm_num + 1,
        attempt + 1
    );
    rename(org_filename, &new_filename).expect("Could not rename file");

    if verbose {
        let ffmpeg_error_line = read_last_line_at(1);
        println!("{}", ffmpeg_error_line.as_str());
        println!("See {} for more details.", new_filename);
    }

    return None;
}

fn check_encode_quality(
    p: &mut Permutation,
    ctrl_channel: &Result<Receiver<()>, Error>,
    verbose: bool,
    perm_num: usize,
) -> Option<c_float> {
    let mut quality_val = None;
    for attempt in 0..MAX_ATTEMPTS_CALC_QUALITY {
        if verbose {
            print!("[ ATTEMPT {}/{} ] ", attempt + 1, MAX_ATTEMPTS_CALC_QUALITY);
        }
        quality_val = calc_vmaf_score(p, &ctrl_channel, verbose, attempt, perm_num);
        match quality_val {
            Some(val) => {
                quality_val = Some(val);
                break;
            }

            None => println!("Check encode quality failed. Retrying..."),
        }
    }

    if quality_val.is_none() {
        panic!(
            "Error, Failed to calc encode quality after {} attempts",
            MAX_ATTEMPTS_CALC_QUALITY
        );
    }

    return quality_val;
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
    let format = if encoder.contains("h264") {
        "h264"
    } else if encoder.contains("hevc") {
        "hevc"
    } else {
        "ivf"
    };
    // this should be cleaner when we support more than 1 type
    return input.replace("{}", format);
}
