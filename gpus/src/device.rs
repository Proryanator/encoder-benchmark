use nvml_wrapper::enum_wrappers::device::Brand;

// our own wrapper for device information to get around lifetime issues
pub struct Device {
    pub brand: Brand,
    pub name: String,
}

impl Device {
    pub fn new() -> Self {
        return Self {
            brand: Brand::Unknown,
            name: String::from("")
        };
    }
}