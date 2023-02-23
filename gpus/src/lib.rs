use nvml_wrapper::Nvml;

pub mod device;

pub fn get_gpus() -> Vec<String> {
    let nvml = Nvml::init().unwrap();
    let device_count = nvml.device_count().unwrap();

    let mut list = Vec::new();

    for i in 0..device_count {
        let nvml_device = nvml.device_by_index(i).unwrap();
        list.push(nvml_device.name().unwrap());
    }

    return list;
}