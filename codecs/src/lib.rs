use crate::vendor::Vendor;

pub mod nvenc;
pub mod amf;
pub mod permute;
mod resolutions;
pub mod vendor;
pub mod qsv;
pub mod av1_qsv;


pub fn get_vendor_for_codec(codec: &String) -> Vendor {
    if codec.contains("nvenc") {
        return Vendor::Nvidia;
    } else if codec.contains("amf") {
        return Vendor::AMD;
    } else if codec.contains("h264_qsv") || codec.contains("hevc_qsv") || codec.contains("av1_qsv") {
        return Vendor::IntelQSV;
    }

    return Vendor::Unknown;
}