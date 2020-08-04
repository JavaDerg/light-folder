use super::*;
use crossbeam_deque::Injector;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tokio::sync::broadcast::{channel, Sender, TryRecvError};

lazy_static! {
    pub(super) static ref WORK_QUEUE: Injector<WorkUnit> = Injector::new();
    static ref THREAD_COUNT: Mutex<usize> = Mutex::new(0usize);
    static ref SHUTDOWN_NOTIFIER: Mutex<Sender<()>> = Mutex::new(channel(1).0);
    static ref THREAD_HANDELS: Mutex<Vec<JoinHandle<()>>> = Mutex::new(vec![]);
}

struct ThreadMonitor(pub usize);

impl ThreadMonitor {
    pub fn new() -> Self {
        use thread_id::get;

        let id = get();
        debug!("@{} Spawned thread", id);
        *THREAD_COUNT.lock().unwrap() += 1;
        Self(id)
    }

    pub fn exit(&self) {
        debug!("@{} Exiting thread", self.0);
        *THREAD_COUNT.lock().unwrap() -= 1;
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

pub fn start_worker_threads() {
    use crate::config::CPU_THREADS;

    info!("Starting image processor");
    debug!("Found usable {} threads", *CPU_THREADS);

    (0..*CPU_THREADS).for_each(|_| {
        let handle = std::thread::spawn(|| new_thread(false));
        THREAD_HANDELS.lock().unwrap().push(handle);
    });
}

pub fn shutdown() {
    info!("Shutting down ImagePool");
    SHUTDOWN_NOTIFIER.lock().unwrap().send(()).unwrap();
    let mut handles = THREAD_HANDELS.lock().unwrap();
    while let Some(th) = handles.pop() {
        let _ = th.join();
    }
}

fn new_thread(from_panic: bool) {
    use crossbeam_deque::{Steal, Worker};

    let monitor = ThreadMonitor::new();
    let mut shutdown_monitor = SHUTDOWN_NOTIFIER.lock().unwrap().subscribe();

    if from_panic {
        warn!("@{} Spawned recovery thread", monitor.0);
    }

    let worker = Worker::<super::WorkUnit>::new_fifo();
    loop {
        match shutdown_monitor.try_recv() {
            Ok(_) | Err(TryRecvError::Closed) => break,
            _ => (),
        }
        match worker.pop() {
            Some(work) => do_work(work),
            None => match WORK_QUEUE.steal_batch_and_pop(&worker) {
                Steal::Success(work) => do_work(work),
                _ => std::thread::yield_now(),
            },
        }
    }
    monitor.exit();
}

fn do_work(work: super::WorkUnit) {
    use opencv::core::Vector;

    let _ = work.responder.send(super::logic::resize_image_sync(
        Vector::from_iter(work.image_data.into_iter()),
        work.width,
        work.height,
        work.target,
    ));
}
