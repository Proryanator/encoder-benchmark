#[derive(Copy, Clone)]
pub struct MetaData {
    pub fps: u32,
    pub frames: u64,
    pub width: u32,
    pub height: u32,
}

impl MetaData {
    pub fn new() -> Self {
        return Self {
            fps: 0,
            frames: 0,
            width: 0,
            height: 0,
        };
    }

    pub fn to_string(&self) -> String {
        return format!(
            "Video metadata: fps: {}, total_frames: {}, resolution: {}x{}",
            self.fps, self.frames, self.width, self.height
        );
    }

    pub fn get_res(&self) -> String {
        return format!("{}x{}", self.width, self.height);
    }

    pub fn is_empty(&self) -> bool {
        return self.frames == 0;
    }
}
