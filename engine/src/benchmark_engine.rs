use std::time::{Duration, SystemTime};

use compound_duration::format_dhms;

use permutation::permutation::Permutation;

use crate::engine::{log_benchmark_header, run_encode};
use crate::result::{log_results_to_file, PermutationResult};
use crate::threads::setup_ctrl_channel;

pub struct BenchmarkEngine {
    permutations: Vec<Permutation>,
    results: Vec<PermutationResult>,
    log_files_directory: String,
}

impl BenchmarkEngine {
    pub fn new(log_files: String) -> Self {
        return Self {
            permutations: vec![],
            results: vec![],
            log_files_directory: log_files,
        };
    }

    pub fn run(&mut self) {
        let runtime = SystemTime::now();
        let ctrl_channel = setup_ctrl_channel();

        let mut calc_time: Option<Duration> = None;
        for i in 0..self.permutations.clone().len() {
            let permutation_start_time = SystemTime::now();
            let permutation = self.permutations[i].clone();
            // benchmark will not log ETA since every encode will be different
            log_benchmark_header(i, &self.permutations, calc_time);
            self.results
                .push(run_encode(permutation.clone(), &ctrl_channel));
            calc_time = Option::from(permutation_start_time.elapsed().unwrap());
        }

        // produce output files and other logging here
        let runtime_str = format_dhms(runtime.elapsed().unwrap().as_secs());
        log_results_to_file(
            self.results.clone(),
            &runtime_str,
            Vec::new(),
            self.permutations[0].bitrate,
            true,
            &self.log_files_directory,
        );
        println!("Benchmark runtime: {}", runtime_str);
    }

    pub fn add(&mut self, permutation: Permutation) {
        self.permutations.push(permutation);
    }
}
