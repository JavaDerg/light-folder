use super::*;
use opencv::core::{Mat, MatTrait, MatTraitManual, Size};
use opencv::types::VectorOfu8;

pub async fn resize_image(
    image_data: Vec<u8>,
    width: u32,
    height: u32,
    target: ImageTarget,
) -> Result<Vec<u8>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    super::work::WORK_QUEUE.push(WorkUnit {
        image_data,
        width,
        height,
        target,
        responder: tx,
    });
    rx.await.unwrap_or_else(|_| Err(Error::OneshotReceiveError))
}

pub(super) fn resize_image_sync(
    image_data: VectorOfu8,
    mut width: u32,
    mut height: u32,
    target: ImageTarget,
) -> Result<Vec<u8>> {
    use opencv::imgproc::{resize, InterpolationFlags};

    let img = load_image(&image_data).map_err(|err| ImageError::ImageLoadingError(err.message))?;
    let isize = img
        .size()
        .map_err(|err| ImageError::GeneralImageError(err.message))?;
    if !verify_and_adjust_scale(
        isize.width as u32,
        isize.height as u32,
        &mut width,
        &mut height,
    ) {
        return Ok(image_data.to_vec());
    };

    if isize.width as u32 == width {
        return Ok(write_image(&img, target)?);
    }

    let size = Size::new(width as i32, height as i32);
    let mut buf = unsafe {
        Mat::new_size(
            size,
            img.typ()
                .map_err(|err| ImageError::GeneralImageError(err.message))?,
        )
    }
    .map_err(|err| ImageError::ImageCreationError(err.message))?;

    resize(&img, &mut buf, size, 0.0, 0.0, unsafe {
        std::mem::transmute(InterpolationFlags::INTER_AREA)
    })
    .map_err(|err| ImageError::ImageResizingError(err.message))?;

    Ok(write_image(&buf, target)?)
}

#[inline]
fn write_image(mat: &Mat, target: ImageTarget) -> Result<Vec<u8>> {
    use opencv::core::Vector;
    use opencv::imgcodecs::{imencode, IMWRITE_WEBP_QUALITY};

    let mut out_vec = Vector::new();
    if let ImageTarget::WebP = target {
        imencode(target.ext(), mat, &mut out_vec, &{
            let mut v = Vector::with_capacity(2);
            v.push(IMWRITE_WEBP_QUALITY);
            v.push(101i32);
            v
        })
        .map_err(|err| ImageError::ImageEncodingError(err.message))?;
    } else {
        imencode(target.ext(), mat, &mut out_vec, &Vector::<i32>::new())
            .map_err(|err| ImageError::ImageEncodingError(err.message))?;
    }

    Ok(out_vec.to_vec())
}

#[inline]
fn load_image(image_data: &VectorOfu8) -> opencv::Result<Mat> {
    use opencv::imgcodecs::{imdecode, ImreadModes};
    use std::mem::transmute;

    imdecode(image_data, unsafe {
        transmute(ImreadModes::IMREAD_UNCHANGED)
    })
}

fn verify_and_adjust_scale(ox: u32, oy: u32, tx: &mut u32, ty: &mut u32) -> bool {
    if *tx > ox || *ty > oy {
        return false;
    }
    if *tx == 0 && *ty == 0 {
        *tx = ox;
        *ty = oy;
        return true;
    }
    let ratio = ox as f64 / oy as f64;
    if *tx == 0 {
        *tx = (*ty as f64 * ratio).floor() as u32;
        true
    } else if *ty == 0 {
        *ty = (*tx as f64 / ratio).floor() as u32;
        true
    } else {
        let target_ratio = *ty as f64 / *tx as f64;
        (ratio - target_ratio).abs() > 0.01 // not using f64::epsilon() due to possible error of converting f64 -> u64
    }
}
