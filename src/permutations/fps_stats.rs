#[derive(Clone)]
pub(crate) struct FpsStats {
    pub(crate) avg: u16,
    pub(crate) one_perc_low: u16,
    pub(crate) ninety_perc: u16,
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