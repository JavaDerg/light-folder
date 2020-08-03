use crossbeam_deque::{Injector, Steal, Worker};
use lazy_static::lazy_static;
use opencv::core::{Mat, MatTrait, MatTraitManual, Size, Vector};
use opencv::imgcodecs::ImreadModes;
use opencv::types::VectorOfu8;
use std::sync::Arc;

use crate::*;

lazy_static! {
    static ref WORK_QUEUE: Arc<Injector<WorkUnit>> = Arc::new(Injector::new());
}

pub fn start() {
    use super::config::CPU_THREADS;

    info!("Starting image processor");
    debug!("Found usable {} threads", *CPU_THREADS);

    for _ in 0..*CPU_THREADS {
        std::thread::spawn(|| new_thread(false));
    }
}

struct ThreadMonitor(pub usize);

impl ThreadMonitor {
    pub fn exit(&self) {
        debug!("@{} Exiting thread", self.0)
    }
}

impl Drop for ThreadMonitor {
    fn drop(&mut self) {
        use std::thread;

        if thread::panicking() {
            error!("@{} Thread panic in ImagePool, trying to recover", self.0);
            thread::spawn(|| new_thread(true));
        }
    }
}

fn new_thread(from_panic: bool) {
    let monitor = ThreadMonitor(thread_id::get());

    debug!("@{} Spawned thread", monitor.0);

    if from_panic {
        warn!("@{} Spawned recovery thread", monitor.0);
    }

    let worker = Worker::<WorkUnit>::new_fifo();
    loop {
        if let Some(work) = worker.pop().or_else(|| {
            if let Steal::Success(work) = WORK_QUEUE.steal_batch_and_pop(&worker) {
                Some(work)
            } else {
                None
            }
        }) {
            do_work(work);
        } else {
            std::thread::yield_now()
        }
    }
    monitor.exit();
}

fn do_work(work: WorkUnit) {
    let _ = work.responder.send(resize_image_sync(
        Vector::from_iter(work.image_data.into_iter()),
        work.width,
        work.height,
        work.target,
    ));
}

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

pub async fn resize_image(
    image_data: Vec<u8>,
    width: u32,
    height: u32,
    target: ImageTarget,
) -> Result<Vec<u8>> {
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    WORK_QUEUE.push(WorkUnit {
        image_data,
        width,
        height,
        target,
        responder: tx,
    });
    rx.await
        .unwrap_or_else(|_| Err(Error::OneshotReceiveError))
}

fn resize_image_sync(
    image_data: VectorOfu8,
    mut width: u32,
    mut height: u32,
    target: ImageTarget,
) -> Result<Vec<u8>> {
    use opencv::imgproc::{resize, InterpolationFlags};

    let img = load_image(&image_data).map_err(|err| ImageError::ImageLoadingError(err.message))?;
    let isize = img.size().map_err(|err| ImageError::GeneralImageError(err.message))?;
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
            img.typ().map_err(|err| ImageError::GeneralImageError(err.message))?,
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
            v.push(100i32);
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
        transmute(ImreadModes::IMREAD_ANYCOLOR)
    }) // FIXME: Potential bug, check if IMREAD_ANYCOLOR supports transparancy!
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
