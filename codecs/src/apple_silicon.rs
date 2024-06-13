use std::collections::HashMap;

use itertools::Itertools;

use crate::permute::Permute;
use crate::resolutions::map_res_to_bitrate;

// TODO: add in values to permutation for -realtime, -prio_speed
// all available encoders, h264, hevc, and prores have similar shared options other than profiles
// note: level does not appear to be supported and throws errors
pub struct Apple {
    profiles: Vec<&'static str>,
    coders: Vec<&'static str>,
    // might be able to make this the size we're expecting
    permutations: Vec<String>,
    is_h264: bool,
    is_prores: bool,
    index: i32,
}

impl Apple {
    // note: h264 has a unique coders option
    pub fn new(is_h264: bool, is_prores: bool) -> Self {
        Self {
            profiles: if is_h264 {
                vec![
                    "baseline",
                    "constrained_baseline",
                    "main",
                    "high",
                    "constrained_high",
                    "extended",
                ]
            } else if is_prores {
                vec!["auto", "proxy", "lt", "standard", "hq", "4444", "xq"]
            } else {
                vec!["main", "main10"]
            },
            // note: only h264 has the coders option
            coders: if is_h264 {
                vec!["vlc", "cavlc", "cabac", "ac"]
            } else {
                vec![]
            },
            permutations: Vec::new(),
            is_h264,
            is_prores,
            // starts at -1, so that first next() will return the first element
            index: -1,
        }
    }

    pub fn get_benchmark_settings(&self) -> String {
        return if self.is_h264 {
            String::from("-profile:v baseline -coder vlc -constant_bit_rate true ")
        } else if self.is_prores {
            String::from("-profile:v auto ")
        } else {
            String::from("-profile:v main -constant_bit_rate true ")
        };
    }

    fn has_next(&self) -> bool {
        return self.index != (self.permutations.len() - 1) as i32;
    }
}

#[derive(Copy, Clone)]
struct AppleSettings {
    profile: &'static str,
    coder: &'static str,
    is_prores: bool,
}

impl AppleSettings {
    fn to_string(&self) -> String {
        let mut args = String::new();
        args.push_str("-profile:v ");
        args.push_str(self.profile);

        // hevc does not support this, so this will be empty for hevc
        if !self.coder.is_empty() {
            args.push_str(" -coder ");
            args.push_str(self.coder);
        }

        // prores does not have/support constant bit rate
        if !self.is_prores {
            args.push_str(" -constant_bit_rate");
            args.push_str(" true");
        }

        return args;
    }
}

impl Iterator for Apple {
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

impl Permute for Apple {
    fn init(&mut self) -> &Vec<String> {
        // reset index, otherwise we won't be able to iterate at all
        self.index = -1;

        // clear the vectors if there were entries before
        self.permutations.clear();

        let vectors = if self.is_h264 {
            vec![&self.profiles, &self.coders]
        } else {
            vec![&self.profiles]
        };
        let mut permutations = vectors.into_iter().multi_cartesian_product();

        loop {
            let perm = permutations.next();
            if perm.is_none() {
                break;
            }

            let unwrapped_perm = perm.unwrap();
            let settings = AppleSettings {
                profile: unwrapped_perm.get(0).unwrap(),
                coder: if self.is_h264 {
                    unwrapped_perm.get(1).unwrap()
                } else {
                    ""
                },
                is_prores: self.is_prores,
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

        // TODO: need to update this for apple silicon
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
    use crate::apple_silicon::Apple;
    use crate::permute::Permute;

    #[test]
    fn create_h264_test() {
        let apple_h264 = Apple::new(true, false);
        assert!(apple_h264.profiles.contains(&"constrained_baseline"));
    }

    #[test]
    fn create_hevc_test() {
        let apple_h264 = Apple::new(false, false);
        assert!(apple_h264.profiles.contains(&"main10"));
    }

    #[test]
    fn create_prores_test() {
        let apple_h264 = Apple::new(false, true);
        assert!(apple_h264.profiles.contains(&"lt"));
    }

    #[test]
    fn iterate_to_end_test() {
        let mut apple = Apple::new(false, false);
        let perm_count = apple.init().len();

        let mut total = 0;
        while let Some((_usize, _string)) = apple.next() {
            total += 1
        }

        // determine if we iterated over all the permutations correctly
        assert_eq!(total, perm_count);
    }
}
