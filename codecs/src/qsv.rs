use std::collections::HashMap;

use itertools::Itertools;

use crate::permute::Permute;
use crate::resolutions::map_res_to_bitrate;

pub struct QSV {
    presets: Vec<&'static str>,
    profiles: Vec<&'static str>,
    // might be able to make this the size we're expecting
    permutations: Vec<String>,
    index: i32,
}

impl QSV {
    pub fn new(is_hevc: bool) -> Self {
        Self {
            presets: get_qsv_presets(),
            // this is the only difference between hevc & h264
            // note: there are more profiles for hevc but, on dev's CPU they were not supported
            profiles: if is_hevc {
                vec!["unknown", "main", "mainsp"]
            } else {
                vec!["unknown", "baseline", "main", "high"]
            },
            permutations: Vec::new(),
            // starts at -1, so that first next() will return the first element
            index: -1,
        }
    }

    pub fn get_benchmark_settings(&self) -> String {
        return String::from("-preset faster -profile main");
    }

    fn has_next(&self) -> bool {
        return self.index != (self.permutations.len() - 1) as i32;
    }
}

fn get_qsv_presets() -> Vec<&'static str> {
    return vec![
        "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow",
    ];
}

#[derive(Copy, Clone)]
struct QSVSettings {
    preset: &'static str,
    profile: &'static str,
}

impl QSVSettings {
    fn to_string(&self) -> String {
        let mut args = String::new();
        args.push_str("-preset ");
        args.push_str(self.preset);
        args.push_str(" -profile:v ");
        args.push_str(self.profile);

        return args;
    }
}

impl Iterator for QSV {
    type Item = (usize, String);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next() {
            return None;
        }

        self.index += 1;

        let usize_index = self.index as usize;
        return Option::from((
            usize_index as usize,
            self.permutations.get(usize_index).unwrap().to_string(),
        ));
    }
}

impl Permute for QSV {
    fn init(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        let mut permutations = vec![&self.presets, &self.profiles]
            .into_iter()
            .multi_cartesian_product();

        loop {
            let perm = permutations.next();
            if perm.is_none() {
                break;
            }

            let unwrapped_perm = perm.unwrap();
            let settings = QSVSettings {
                preset: unwrapped_perm.get(0).unwrap(),
                profile: unwrapped_perm.get(1).unwrap(),
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
        self.permutations
            .push(String::from(self.get_benchmark_settings()));
        return &self.permutations;
    }

    fn get_resolution_to_bitrate_map(fps: u32) -> HashMap<String, u32> {
        let mut map: HashMap<String, u32> = HashMap::new();

        // bitrates are within 5Mb/s of each other, using higher one
        // note: these are the 60fps bitrate values
        // NOTE: these bitrates might not apply for Arc GPU's H264/HEVC encoders
        let mut bitrates: [u32; 4] = [20, 30, 35, 70];

        // 120 fps is effectively double the bitrate
        if fps == 120 {
            bitrates.iter_mut().for_each(|b| *b = *b * 2);
        }

        map_res_to_bitrate(&mut map, bitrates);

        return map;
    }
}
