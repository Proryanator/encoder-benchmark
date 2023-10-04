use std::collections::HashMap;

pub const SUPPORTED_RESOLUTIONS: [&'static str; 4] =
    ["1280x720", "1920x1080", "2560x1440", "3840x2160"];

pub fn map_res_to_bitrate(map: &mut HashMap<String, u32>, bitrates: [u32; 4]) {
    map.insert(
        SUPPORTED_RESOLUTIONS.get(0).unwrap().to_string(),
        *bitrates.get(0).unwrap(),
    );
    map.insert(
        SUPPORTED_RESOLUTIONS.get(1).unwrap().to_string(),
        *bitrates.get(1).unwrap(),
    );
    map.insert(
        SUPPORTED_RESOLUTIONS.get(2).unwrap().to_string(),
        *bitrates.get(2).unwrap(),
    );
    map.insert(
        SUPPORTED_RESOLUTIONS.get(3).unwrap().to_string(),
        *bitrates.get(3).unwrap(),
    );
}
