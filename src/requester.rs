use crate::img::ImageTarget;
use isahc::{get_async, head_async};
use lazy_static::lazy_static;
use std::io::Read;

lazy_static! {}

// TODO: Add proper user-agent
pub async fn obtain_image(path: &str) -> Result<(Vec<u8>, ImageTarget), String> {
    println!("Requesting '{}'", path);
    let head = head_async(path).await.map_err(|err| format!("{}", err))?;
    let headers = head.headers();
    let content_type =
        std::str::from_utf8(headers.get("content-type").unwrap().as_bytes()).unwrap(); // TODO: add error handling
    let content_length = std::str::from_utf8(headers.get("content-length").unwrap().as_bytes())
        .unwrap()
        .parse::<u64>()
        .unwrap();

    if !content_type.starts_with("image/") {
        return Err("Received data is not a image".to_string());
    }

    if content_length > 50_000_000 {
        // TODO: make this a env variable
        return Err("Received image is to big".to_string());
    }
    let image = get_async(path)
        .await
        .unwrap()
        .into_body()
        .bytes()
        .map(|b| b.unwrap())
        .collect();

    Ok((image, ImageTarget::parse_or_default(&content_type[6..])))
}
