use std::collections::HashMap;

use itertools::Itertools;

use crate::permute::Permute;
use crate::resolutions::map_res_to_bitrate;

pub struct Amf {
    usages: Vec<&'static str>,
    qualities: Vec<&'static str>,
    profiles: Vec<&'static str>,
    profile_tiers: Vec<&'static str>,
    rate_controls: Vec<&'static str>,
    // might be able to make this the size we're expecting
    permutations: Vec<String>,
    index: i32,
    gpu: u8,
}

impl Amf {
    pub fn new(is_hevc: bool, gpu: u8) -> Self {
        Self {
            usages: get_amf_usages(),
            qualities: get_amf_quality(),
            // this is the only difference between hevc & h264
            profiles: if is_hevc {
                vec!["main"]
            } else {
                vec!["main", "high", "constrained_baseline", "constrained_high"]
            },
            // leaving out vbr rate controls as these are not ideal for game streaming
            profile_tiers: get_amf_profile_tiers(is_hevc),
            rate_controls: vec!["cbr"],
            permutations: Vec::new(),
            // starts at -1, so that first next() will return the first element
            index: -1,
            gpu,
        }
    }

    pub fn get_benchmark_settings(&self) -> String {
        // both hevc and h264 perform best at main (kinda, h264 it doesn't matter much)
        // hevc and h264 both share the same high fps with the same settings, even without profile_tier for hevc
        let profile = "main";
        return format!(
            "-usage ultralowlatency -quality speed -profile:v {} -rc cbr -cbr true -gpu {}",
            profile, self.gpu
        );
    }

    fn has_next(&self) -> bool {
        return self.index != (self.permutations.len() - 1) as i32;
    }
}

fn get_amf_profile_tiers(hevc: bool) -> Vec<&'static str> {
    if hevc {
        return vec!["main", "high"];
    }

    // there are no amf profile tiers for h264
    return vec![];
}

fn get_amf_usages() -> Vec<&'static str> {
    return vec!["transcoding", "ultralowlatency", "lowlatency", "webcam"];
}

fn get_amf_quality() -> Vec<&'static str> {
    return vec!["balanced", "speed", "quality"];
}

#[derive(Copy, Clone)]
struct AmfSettings {
    usage: &'static str,
    quality: &'static str,
    profile: &'static str,
    profile_tier: &'static str,
    rate_control: &'static str,
    gpu: u8,
}

impl AmfSettings {
    fn to_string(&self) -> String {
        let mut args = String::new();
        args.push_str("-usage ");
        args.push_str(self.usage);
        args.push_str(" -quality ");
        args.push_str(self.quality);
        args.push_str(" -profile:v ");
        args.push_str(self.profile);

        if !self.profile_tier.is_empty() {
            args.push_str(" -profile_tier ");
            args.push_str(self.profile_tier);
        }

        args.push_str(" -rc ");
        args.push_str(self.rate_control);
        // always set this to constant bit rate to ensure reliable stream
        args.push_str(" -cbr true");
        args.push_str(" -gpu ");
        args.push_str(self.gpu.to_string().as_str());

        return args;
    }
}

impl Iterator for Amf {
    type Item = (usize, String);

    // maybe we can pull this code out
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

impl Permute for Amf {
    fn init(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        let mut permutations = if self.profile_tiers.is_empty() {
            vec![
                &self.usages,
                &self.qualities,
                &self.profiles,
                &self.rate_controls,
            ]
        } else {
            vec![
                &self.usages,
                &self.qualities,
                &self.profiles,
                &self.profile_tiers,
                &self.rate_controls,
            ]
        }
        .into_iter()
        .multi_cartesian_product();

        loop {
            let perm = permutations.next();
            if perm.is_none() {
                break;
            }

            let unwrapped_perm = perm.unwrap();
            let profile_tier = if !self.profile_tiers.is_empty() {
                unwrapped_perm.get(3).unwrap()
            } else {
                ""
            };
            let rc_index = if !self.profile_tiers.is_empty() { 4 } else { 3 };
            let settings = AmfSettings {
                usage: unwrapped_perm.get(0).unwrap(),
                quality: unwrapped_perm.get(1).unwrap(),
                profile: unwrapped_perm.get(2).unwrap(),
                profile_tier,
                rate_control: unwrapped_perm.get(rc_index).unwrap(),
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
        self.permutations
            .push(String::from(self.get_benchmark_settings()));
        return &self.permutations;
    }

    fn get_resolution_to_bitrate_map(fps: u32) -> HashMap<String, u32> {
        let mut map: HashMap<String, u32> = HashMap::new();

        // bitrates are within 5Mb/s of each other, using higher one
        // note: these are the 60fps bitrate values
        let mut bitrates: [u32; 4] = [20, 35, 50, 85];

        // 120 fps is effectively double the bitrate
        if fps == 120 {
            bitrates.iter_mut().for_each(|b| *b = *b * 2);
        }

        map_res_to_bitrate(&mut map, bitrates);

        return map;
    }
}
