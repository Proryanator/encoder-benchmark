use std::collections::HashMap;

use itertools::Itertools;

use crate::permute::Permute;
use crate::resolutions::map_res_to_bitrate;

// we'll add more options when we add in extended permutation support
pub struct AV1QSV {
    presets: Vec<&'static str>,
    profiles: Vec<&'static str>,
    async_depth: Vec<&'static str>,
    // might be able to make this the size we're expecting
    permutations: Vec<String>,
    index: i32,
}

impl AV1QSV {
    pub fn new() -> Self {
        Self {
            presets: get_qsv_presets(),
            profiles: vec!["main"],
            // anything lower than 4 you get less fps performance, and anything higher than 4 you don't see much return
            // (maybe 1% lows might be a bit higher by a few fps)
            async_depth: vec!["4"],
            permutations: Vec::new(),
            // starts at -1, so that first next() will return the first element
            index: -1,
        }
    }

    pub fn get_benchmark_settings(&self) -> String {
        return String::from("-preset veryfast -profile:v main");
    }

    fn has_next(&self) -> bool {
        return self.index != (self.permutations.len() - 1) as i32;
    }
}

fn get_qsv_presets() -> Vec<&'static str> {
    return vec!["veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"];
}

#[derive(Copy, Clone)]
struct AV1QSVSettings {
    preset: &'static str,
    profile: &'static str,
    async_depth: &'static str,
}

impl AV1QSVSettings {
    fn to_string(&self) -> String {
        let mut args = String::new();
        args.push_str("-preset ");
        args.push_str(self.preset);
        args.push_str(" -profile:v ");
        args.push_str(self.profile);
        args.push_str(" -async_depth ");
        args.push_str(self.async_depth);

        return args;
    }
}

impl Iterator for AV1QSV {
    type Item = (usize, String);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next() {
            return None;
        }

        self.index += 1;

        let usize_index = self.index as usize;
        return Option::from((usize_index as usize, self.permutations.get(usize_index).unwrap().to_string()));
    }
}

impl Permute for AV1QSV {
    fn init(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        let mut permutations = vec![&self.presets, &self.profiles, &self.async_depth]
            .into_iter().multi_cartesian_product();

        loop {
            let perm = permutations.next();
            if perm.is_none() {
                break;
            }

            let unwrapped_perm = perm.unwrap();
            let settings = AV1QSVSettings {
                preset: unwrapped_perm.get(0).unwrap(),
                profile: unwrapped_perm.get(1).unwrap(),
                async_depth: unwrapped_perm.get(2).unwrap(),
            };

            self.permutations.push(settings.to_string());
        }

        return &self.permutations;
    }

    fn run_standard_only(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        // note: this only works when hevc/h264 both use just 1 profile, if we add more this will break
        self.permutations.push(String::from(self.get_benchmark_settings()));
        return &self.permutations;
    }

    fn get_resolution_to_bitrate_map(fps: u32) -> HashMap<String, u32> {
        let mut map: HashMap<String, u32> = HashMap::new();

        // bitrates are within 5Mb/s of each other, using higher one
        // note: these are the 60fps bitrate values
        // TODO: add in bitrate values here after running the tool
        let mut bitrates: [u32; 4] = [20, 30, 35, 70];

        // 120 fps is effectively double the bitrate
        if fps == 120 {
            bitrates.iter_mut().for_each(|b| *b = *b * 2);
        }

        map_res_to_bitrate(&mut map, bitrates);

        return map;
    }
}