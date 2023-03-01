use std::collections::HashMap;

use itertools::Itertools;

use crate::permute::Permute;
use crate::resolutions::map_res_to_bitrate;

pub struct Nvenc {
    presets: Vec<&'static str>,
    tunes: Vec<&'static str>,
    profiles: Vec<&'static str>,
    rate_controls: Vec<&'static str>,
    // might be able to make this the size we're expecting
    permutations: Vec<String>,
    index: i32,
    gpu: u8,
}

impl Nvenc {
    pub fn new(is_hevc: bool, gpu: u8) -> Self {
        Self {
            presets: get_nvenc_presets(),
            tunes: get_nvenc_tunes(),
            // this is the only difference between hevc & h264
            profiles: if is_hevc { vec!["main"] } else { vec!["high"] },
            // leaving out vbr rate controls as these are not ideal for game streaming
            rate_controls: vec!["cbr"],
            permutations: Vec::new(),
            // starts at -1, so that first next() will return the first element
            index: -1,
            gpu,
        }
    }

    pub fn get_benchmark_settings(&self) -> String {
        return format!("-preset p1 -tune ll -profile:v {} -rc cbr -cbr true -gpu {}", self.profiles.get(0).unwrap(), self.gpu);
    }

    fn has_next(&self) -> bool {
        return self.index != (self.permutations.len() - 1) as i32;
    }
}

fn get_nvenc_presets() -> Vec<&'static str> {
    return vec!["p1", "p2", "p3", "p4", "p5", "p6", "p7"];
}

fn get_nvenc_tunes() -> Vec<&'static str> {
    return vec!["hq", "ll", "ull"];
}

#[derive(Copy, Clone)]
struct NvencSettings {
    preset: &'static str,
    tune: &'static str,
    profile: &'static str,
    rate_control: &'static str,
    gpu: u8,
}

impl NvencSettings {
    fn to_string(&self) -> String {
        let mut args = String::new();
        args.push_str("-preset ");
        args.push_str(self.preset);
        args.push_str(" -tune ");
        args.push_str(self.tune);
        args.push_str(" -profile:v ");
        args.push_str(self.profile);
        args.push_str(" -rc ");
        args.push_str(self.rate_control);
        // always set this to constant bit rate to ensure reliable stream
        args.push_str(" -cbr true");
        args.push_str(" -gpu ");
        args.push_str(self.gpu.to_string().as_str());

        return args;
    }
}

impl Iterator for Nvenc {
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

impl Permute for Nvenc {
    fn init(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        let mut permutations = vec![&self.presets, &self.tunes, &self.profiles, &self.rate_controls]
            .into_iter().multi_cartesian_product();

        loop {
            let perm = permutations.next();
            if perm.is_none() {
                break;
            }

            let unwrapped_perm = perm.unwrap();
            let settings = NvencSettings {
                preset: unwrapped_perm.get(0).unwrap(),
                tune: unwrapped_perm.get(1).unwrap(),
                profile: unwrapped_perm.get(2).unwrap(),
                rate_control: unwrapped_perm.get(3).unwrap(),
                gpu: self.gpu,
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
        let mut bitrates: [u32; 4] = [10, 20, 25, 55];

        // 120 fps is effectively double the bitrate
        if fps == 120 {
            bitrates.iter_mut().for_each(|b| *b = *b * 2);
        }

        map_res_to_bitrate(&mut map, bitrates);

        return map;
    }
}

#[cfg(test)]
mod tests {
    use crate::nvenc::Nvenc;
    use crate::permute::Permute;

    #[test]
    fn create_h264_test() {
        let nvenc = Nvenc::new(false, 0);
        assert!(nvenc.profiles.contains(&"high"));
    }

    #[test]
    fn create_hevc_test() {
        let nvenc = Nvenc::new(true, 0);
        assert!(nvenc.profiles.contains(&"main"));
    }

    #[test]
    fn iterate_to_end_test() {
        let mut nvenc = Nvenc::new(false, 0);
        let perm_count = nvenc.init().len();

        let mut total = 0;
        while let Some((_usize, _string)) = nvenc.next() {
            total += 1
        }

        // determine if we iterated over all the permutations correctly
        assert_eq!(total, perm_count);
    }

    #[test]
    fn total_permutations_test() {
        let mut nvenc = Nvenc::new(false, 0);
        assert_eq!(nvenc.init().len(), get_expected_len(&nvenc));
    }

    #[test]
    fn init_twice_not_double_test() {
        let mut nvenc = Nvenc::new(false, 0);
        nvenc.init();
        assert_eq!(nvenc.init().len(), get_expected_len(&nvenc));
    }

    fn get_expected_len(nvenc: &Nvenc) -> usize {
        return nvenc.presets.len() * nvenc.tunes.len() * nvenc.profiles.len() * nvenc.rate_controls.len();
    }
}