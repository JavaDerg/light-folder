use crate::img::ImageTarget;
use saphir::prelude::*;
use std::sync::atomic::AtomicU8;

pub struct ProxyController {
    _health: AtomicU8,
}

impl ProxyController {
    pub fn new<S: Into<String>>(label: S) -> Self {
        Self {
            _health: AtomicU8::new(0),
        }
    }
}

impl Controller for ProxyController {
    const BASE_PATH: &'static str = "/proxy";

    fn handlers(&self) -> Vec<ControllerEndpoint<Self>>
    where
        Self: Sized,
    {
        let b = EndpointsBuilder::new();

        b.add(Method::GET, "/img/{path}", Self::proxy_image).build()
    }
}

impl ProxyController {
    pub async fn proxy_image(&self, mut req: Request) -> (u16, Result<Vec<u8>, String>) {
        let path = if let Some(path) = req.captures_mut().remove("path") {
            percent_encoding::percent_decode_str(&path)
                .decode_utf8_lossy()
                .to_string() // TODO: catch faulty utf8
        } else {
            return (400, Err("No path supplied".to_string()));
        };
        let query = qstring::QString::from(req.uri().query().unwrap_or_else(|| ""));

        let img = super::requester::obtain_image(&path).await.unwrap();
        let img = super::img::resize_image(
            img.0,
            query
                .get("width")
                .map(|n| n.parse().unwrap())
                .unwrap_or(0u32), // TODO: remove unwrap here!
            query
                .get("height")
                .map(|n| n.parse().unwrap())
                .unwrap_or(0u32),
            if let Some(format) = query.get("format") {
                ImageTarget::parse_or_default(format)
            } else {
                img.1
            },
        )
        .await
        .unwrap();
        (200, Ok(img))
    }
}
