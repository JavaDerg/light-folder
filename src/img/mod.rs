pub(self) mod logic;
pub(self) mod work;

use crate::*;

pub use logic::resize_image;
pub use work::start_worker_threads;

struct WorkUnit {
    image_data: Vec<u8>,
    width: u32,
    height: u32,
    target: ImageTarget,
    responder: tokio::sync::oneshot::Sender<Result<Vec<u8>>>,
}

#[derive(Copy, Clone, Debug)]
pub enum ImageTarget {
    Png,
    Jpeg,
    WebP,
    WebPLQ,
}

impl ImageTarget {
    pub fn ext(&self) -> &'static str {
        match self {
            ImageTarget::Png => ".png",
            ImageTarget::Jpeg => ".jpg",
            ImageTarget::WebPLQ | ImageTarget::WebP => ".webp",
        }
    }

    pub fn parse_or_default(s: &str) -> Self {
        match s {
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "webp" => Self::WebP,
            _ => Self::WebPLQ,
        }
    }
}

pub fn shutdown() {
    info!("Shutting down image module");
    work::shutdown();
}
