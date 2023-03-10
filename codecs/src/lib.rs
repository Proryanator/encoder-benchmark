use crate::vendor::Vendor;

pub mod nvenc;
pub mod amf;
pub mod permute;
mod resolutions;
pub mod vendor;


pub fn get_vendor_for_codec(codec: &String) -> Vendor {
    if codec.contains("nvenc") {
        return Vendor::Nvidia;
    } else if codec.contains("amf") {
        return Vendor::AMD;
    }

    return Vendor::Unknown;
}