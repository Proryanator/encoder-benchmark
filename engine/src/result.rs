use std::ffi::c_float;
use std::fs::File;
use std::io::Write;

use compound_duration::format_dhms;

use ffmpeg::metadata::MetaData;

use crate::fps_stats::FpsStats;

#[derive(Clone)]
pub struct PermutationResult {
    pub encoder: String,
    pub was_overloaded: bool,
    bitrate: u32,
    metadata: MetaData,
    pub encoder_settings: String,
    // only if the encodes were successful
    pub encode_time: u64,
    pub vmaf_calculation_time: u64,
    pub vmaf_score: c_float,
    pub fps_stats: FpsStats,
}

impl PermutationResult {
    pub fn new(metadata: &MetaData, bitrate: u32, encoder_settings: &String, encoder: &str) -> Self {
        Self {
            encoder: String::from(encoder),
            was_overloaded: false,
            bitrate,
            metadata: metadata.clone(),
            encoder_settings: encoder_settings.to_string(),
            encode_time: 0,
            vmaf_calculation_time: 0,
            vmaf_score: 0.0,
            fps_stats: FpsStats::default(),
        }
    }

    fn to_string(&self) -> String {
        let mut default = String::new();

        let overloaded_indicator = if self.was_overloaded { "[O]" } else { "   " };
        default.push_str(format!("{}{}x{}\t{}\t{}Mb/s", overloaded_indicator, self.metadata.width,
                                 self.metadata.height, self.metadata.fps, self.bitrate).as_str());

        // adjust tabs based on expected vmaf score, or lack of one
        let vmaf_score_str = if self.was_overloaded { format!("{:.5}\t\t", self.vmaf_score) } else if self.vmaf_score != 0.0 { format!("{:.5}\t", self.vmaf_score) } else { format!("0.00000\t\t") };

        default.push_str(format!("\t\t{}\t\t{}\t\t{}{:.0}\t\t{}\t\t{}\t\t{}",
                                 format_dhms(self.encode_time), format_dhms(self.vmaf_calculation_time), vmaf_score_str,
                                 self.fps_stats.avg, self.fps_stats.one_perc_low, self.fps_stats.ninety_perc, self.encoder_settings).as_str());

        return default;
    }
}

pub fn log_results_to_file(results: Vec<PermutationResult>, runtime_str: &String, dup_results: Vec<PermutationResult>, bitrate: u32, is_standard: bool) {
    // might make this naming here more robust eventually
    let first_metadata = results.get(0).unwrap().metadata;
    let encoder = results.get(0).unwrap().encoder.as_str();
    let permute_file_name = format!("{}-{}-{}.log", encoder, first_metadata.get_res(), first_metadata.fps).to_string();
    let benchmark_file_name = format!("{}-benchmark.log", encoder).to_string();
    let file_name = if is_standard { benchmark_file_name } else { permute_file_name };

    let mut w = File::create(file_name).unwrap();

    writeln!(&mut w, "Results from entire permutation:").unwrap();
    writeln!(&mut w, "==================================================================================================================================================================").unwrap();
    writeln!(&mut w, "   [Resolution]\t[FPS]\t[Bitrate]\t[Encode Time]\t[VMAF Time]\t[VMAF Score]\t[Average FPS]\t[1%'ile]\t[90%'ile]\t[Encoder Settings]").unwrap();
    for result in &results {
        writeln!(&mut w, "{}", result.to_string()).unwrap();
    }
    writeln!(&mut w, "==================================================================================================================================================================").unwrap();
    writeln!(&mut w, "Benchmark runtime: {}\n", runtime_str).unwrap();

    let has_logged_dup_header = false;

    // log out the duplicated results so we can keep track of them
    let initial_perms: Vec<PermutationResult> = results
        .into_iter()
        .filter(|res| res.bitrate == bitrate)
        .collect();

    // for each of these, collect the duplicates with the same score
    for perm in initial_perms {
        let moved = &dup_results;
        let dups: Vec<&PermutationResult> = moved
            .into_iter()
            .filter(|res| res.vmaf_score == perm.vmaf_score)
            .collect();

        // only log entries of duplicates
        if dups.is_empty() {
            continue;
        }

        if !has_logged_dup_header {
            writeln!(&mut w, "Encoder settings that produced identical scores:").unwrap();
            writeln!(&mut w, "==================================================================================================================================================================").unwrap();
        }

        writeln!(&mut w, "Identical score: {}", perm.vmaf_score).unwrap();
        writeln!(&mut w, "\tEncoded: [{}]", perm.encoder_settings).unwrap();

        for dup in dups {
            writeln!(&mut w, "\tIgnored: [{}]", dup.encoder_settings).unwrap();
        }

        writeln!(&mut w, "\n").unwrap();
    }

    writeln!(&mut w, "==================================================================================================================================================================").unwrap();
}