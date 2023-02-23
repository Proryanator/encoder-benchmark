use std::ffi::c_float;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time;
use std::time::SystemTime;

use crossbeam_channel::Receiver;
use ctrlc::Error;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

use crate::stat_tcp_listener::start_listening_to_ffmpeg_stats;
use crate::threads::exit_on_ctrl_c;

pub struct TrialResult {
    pub all_fps: Vec<u16>,
    pub was_overloaded: bool,
    pub ffmpeg_error: bool,
}

impl Default for TrialResult {
    fn default() -> Self {
        TrialResult {
            all_fps: vec![],
            was_overloaded: false,
            ffmpeg_error: false,
        }
    }
}

pub fn watch_encode_progress(total_frames: u64, detect_overload: bool, target_fps: u32, verbose: bool, stats_period: c_float, ctrl_channel: &Result<Receiver<()>, Error>) -> TrialResult {
    static FRAME: AtomicUsize = AtomicUsize::new(0);
    static PREVIOUS_FRAME: AtomicUsize = AtomicUsize::new(0);

    // keep track of all fps metrics to calculate on later on
    let mut trial_result = TrialResult::default();
    let bar = ProgressBar::new(total_frames);
    set_bar_style(&bar, "green");
    bar.tick();

    // time it takes for the encoder to need to process the target # of frames
    let overload_time = time::Duration::from_secs(5);
    let allowed_ffmpeg_downtime = time::Duration::from_secs(10);

    let mut checking_overload = false;
    let mut first_overload_detected = SystemTime::now();

    let mut ffmpeg_error_check_time = SystemTime::now();
    let mut checking_ffmpeg_error = false;

    // how many milliseconds has passed since the last frame stat
    let interval_adjustment = (1.0 / stats_period) as usize;

    let stat_listener = start_listening_to_ffmpeg_stats(verbose, &FRAME, &PREVIOUS_FRAME);

    let mut last_frame = 0;
    loop {
        // important to not get stuck in this thread
        exit_on_ctrl_c(&ctrl_channel);

        // takes into account the stat update period to properly adjust the calculated FPS
        let calculated_fps = ((FRAME.load(Ordering::Relaxed) - PREVIOUS_FRAME.load(Ordering::Relaxed)) * interval_adjustment) as u16;

        // only record fps counts that are close to 1/4 of the target; any lower is noise
        if calculated_fps >= (target_fps / 4) as u16 {
            trial_result.all_fps.push(calculated_fps);
        }

        // calculate the number of frames processed since the last second (more accurate than using fps from ffmpeg)
        if detect_overload && calculated_fps < target_fps as u16 {
            if !checking_overload {
                first_overload_detected = SystemTime::now();
                checking_overload = true;
            }

            // check elapsed time since the last encoder overload detection
            if checking_overload && first_overload_detected.elapsed().unwrap() > overload_time {
                break;
            }
        } else {
            checking_overload = false;
        }

        if FRAME.load(Ordering::Relaxed) >= total_frames as usize {
            bar.set_position(total_frames);
            break;
        }

        let new_frame = FRAME.load(Ordering::Relaxed) as u64;
        bar.set_position(new_frame);

        if new_frame != last_frame {
            last_frame = new_frame;
            checking_ffmpeg_error = false;
        } else {
            if !checking_ffmpeg_error {
                // start a timer to detect if there has been an ffmpeg error (unlikely but just in case)
                ffmpeg_error_check_time = SystemTime::now();
                checking_ffmpeg_error = true;
            }

            // check whether we've sat for too long without any progress from ffmpeg
            if ffmpeg_error_check_time.elapsed().unwrap() > allowed_ffmpeg_downtime {
                trial_result.ffmpeg_error = true;
                break;
            }
        }
    }

    // change bar style as read
    if (FRAME.load(Ordering::Relaxed) as u64) < total_frames {
        set_bar_style(&bar, "red");
        bar.abandon()
    } else {
        bar.finish();
    }

    println!();

    // kill the tcp reading thread
    stat_listener.stop().join().expect("Child thread reading TCP did not finish");

    trial_result.was_overloaded = (FRAME.load(Ordering::Relaxed) as u64) != total_frames;
    // reset the static values
    FRAME.store(0, Ordering::Relaxed);
    PREVIOUS_FRAME.store(0, Ordering::Relaxed);

    return trial_result;
}

pub fn set_bar_style(bar: &ProgressBar, color: &str) {
    let template = "{spinner:.%} [{elapsed_precise}] [{wide_bar:.%}] {pos}/{len} frames ({eta_precise})";
    bar.set_style(ProgressStyle::with_template(&str::replace(template, "%", color).as_str())
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
}

pub fn draw_yellow_bar(total_frames: u64) {
    let bar = ProgressBar::new(total_frames);
    set_bar_style(&bar, "yellow");
    bar.tick();
    bar.abandon();
}