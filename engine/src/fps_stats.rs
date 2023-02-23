#[derive(Clone)]
pub struct FpsStats {
    pub avg: u16,
    pub one_perc_low: u16,
    pub ninety_perc: u16,
}

impl Default for FpsStats {
    fn default() -> Self {
        FpsStats {
            avg: 0,
            one_perc_low: 0,
            ninety_perc: 0,
        }
    }
}