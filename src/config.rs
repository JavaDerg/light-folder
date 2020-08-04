use lazy_static::lazy_static;

lazy_static! {
    pub static ref INTERFACES: Vec<String> = get_interfaces();
    pub static ref CPU_THREADS: usize = get_cpu_threads();
}

fn get_interfaces() -> Vec<String> {
    std::env::vars()
        .filter(|(key, _)| key.starts_with("LF_INTERFACE_"))
        .map(|(_, v)| v)
        .collect()
}

fn get_cpu_threads() -> usize {
    std::env::var("LF_IMAGE_THREADS")
        .map(|n| {
            (&n).parse().unwrap_or_else(|_| {
                super::crash(format!(
                    "Expected number on environment variable 'LF_IMAGE_THREADS', found '{}'",
                    &n
                ))
            })
        })
        .unwrap_or_else(|_| num_cpus::get())
}
