#[derive(Copy, Clone)]
pub(crate) struct MetaData {
    pub(crate) fps: u32,
    pub(crate) frames: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl MetaData {
    pub(crate) fn to_string(&self) -> String {
        return format!("Video metadata: fps: {}, total_frames: {}, resolution: {}x{}", self.fps, self.frames, self.width, self.height);
    }

    pub(crate) fn get_res(&self) -> String {
        return format!("{}x{}", self.width, self.height);
    }
}