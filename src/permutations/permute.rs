use std::collections::HashMap;

pub(crate) trait Permute: Iterator {
    // calculates permutations and returns a reference of said permutations
    fn init(&mut self) -> &Vec<String>;

    // overwrites the init values to just include permutations of standard runs
    fn run_standard_only(&mut self) -> &Vec<String>;

    // takes in the fps being used; scales the necessary bitrate accordingly
    fn get_resolution_to_bitrate_map(&self, fps: u32) -> HashMap<String, u32>;
}