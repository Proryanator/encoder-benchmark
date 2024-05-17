use crate::vendor::Vendor;

pub mod amf;
pub mod av1_qsv;
pub mod nvenc;
pub mod permute;
pub mod qsv;
mod resolutions;
pub mod vendor;
pub mod apple_silicon;

pub fn get_vendor_for_codec(codec: &String) -> Vendor {
    if codec.contains("nvenc") {
        return Vendor::Nvidia;
    } else if codec.contains("amf") {
        return Vendor::AMD;
    } else if codec.contains("h264_qsv") || codec.contains("hevc_qsv") || codec.contains("av1_qsv")
    {
        return Vendor::IntelQSV;
    } else if codec.contains("videotoolbox") {
        return Vendor::Apple;
    }

    return Vendor::Unknown;
}
