use rustc_hash::FxHashMap;
use std::any::Any;
use std::cell::RefCell;
use std::future::Future;
use std::sync::mpsc::{self, Receiver};

#[cfg(not(target_arch = "wasm32"))]
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

type ErasedResult = Box<dyn Any + Send>;
type CompletionCallback = Box<dyn FnOnce(ErasedResult)>;

#[cfg(not(target_arch = "wasm32"))]
fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_owned(),
            Err(_) => "non-string panic payload".to_owned(),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn native_waker_clone(data: *const ()) -> RawWaker {
    let thread = (&*(data as *const std::thread::Thread)).clone();
    let boxed = Box::new(thread);
    RawWaker::new(Box::into_raw(boxed) as *const (), &NATIVE_WAKER_VTABLE)
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn native_waker_wake(data: *const ()) {
    let thread = Box::from_raw(data as *mut std::thread::Thread);
    thread.unpark();
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn native_waker_wake_by_ref(data: *const ()) {
    let thread = &*(data as *const std::thread::Thread);
    thread.unpark();
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn native_waker_drop(data: *const ()) {
    let _ = Box::from_raw(data as *mut std::thread::Thread);
}

#[cfg(not(target_arch = "wasm32"))]
static NATIVE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    native_waker_clone,
    native_waker_wake,
    native_waker_wake_by_ref,
    native_waker_drop,
);

#[cfg(not(target_arch = "wasm32"))]
fn block_on_native<F: Future>(future: F) -> F::Output {
    let thread = std::thread::current();
    let raw_waker = RawWaker::new(
        Box::into_raw(Box::new(thread)) as *const (),
        &NATIVE_WAKER_VTABLE,
    );
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

enum JobMessage {
    Completed(ErasedResult),
    #[cfg(not(target_arch = "wasm32"))]
    Panicked(String),
}

struct JobEntry {
    receiver: Receiver<JobMessage>,
    on_complete: Option<CompletionCallback>,
}

pub(crate) struct JobManager {
    jobs: FxHashMap<String, JobEntry>,
}

impl JobManager {
    fn new() -> Self {
        Self {
            jobs: FxHashMap::default(),
        }
    }
}

thread_local! {
    pub(crate) static JOB_MANAGER: RefCell<JobManager> = RefCell::new(JobManager::new());
}

/// Spawn a background job.
///
/// The job executes in the background and `on_complete` is called on the main thread
/// during `ply.begin()` after the job finishes.
pub fn spawn<T, J, Fut, C>(
    id: impl Into<String>,
    job: J,
    on_complete: C,
) -> Result<(), String>
where
    T: Send + 'static,
    J: FnOnce() -> Fut + 'static,
    Fut: Future<Output = T> + Send + 'static,
    C: FnOnce(T) + 'static,
{
    let id = id.into();
    if id.trim().is_empty() {
        return Err("Job id cannot be empty".to_owned());
    }

    let (sender, receiver) = mpsc::channel::<JobMessage>();

    let callback: CompletionCallback = Box::new(move |result| {
        let result = *result
            .downcast::<T>()
            .expect("jobs callback type mismatch");
        on_complete(result);
    });

    let inserted = JOB_MANAGER.with(|manager| {
        let mut manager = manager.borrow_mut();
        if manager.jobs.contains_key(&id) {
            return false;
        }

        manager.jobs.insert(
            id.clone(),
            JobEntry {
                receiver,
                on_complete: Some(callback),
            },
        );

        true
    });

    if !inserted {
        return Err(format!("Job '{}' is already running", id));
    }

    let future = job();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                block_on_native(future)
            }));

            let message = match outcome {
                Ok(result) => JobMessage::Completed(Box::new(result)),
                Err(payload) => JobMessage::Panicked(panic_payload_to_string(payload)),
            };

            let _ = sender.send(message);
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        macroquad::experimental::coroutines::start_coroutine(async move {
            let result = future.await;
            let _ = sender.send(JobMessage::Completed(Box::new(result)));
        });
    }

    Ok(())
}

/// Returns `true` if a job with this ID is currently running.
pub fn running(id: impl AsRef<str>) -> bool {
    let id = id.as_ref();
    JOB_MANAGER.with(|manager| manager.borrow().jobs.contains_key(id))
}

/// Alias of `running` for readability.
pub fn is_running(id: impl AsRef<str>) -> bool {
    running(id)
}

/// Returns all currently running job IDs.
pub fn list() -> Vec<String> {
    JOB_MANAGER.with(|manager| {
        let mut jobs = manager.borrow().jobs.keys().cloned().collect::<Vec<_>>();
        jobs.sort_unstable();
        jobs
    })
}

pub(crate) fn poll_completions() {
    let mut completions: Vec<(CompletionCallback, ErasedResult)> = Vec::new();
    let mut panic_info: Option<(String, String)> = None;

    JOB_MANAGER.with(|manager| {
        let mut manager = manager.borrow_mut();
        let mut finished_ids = Vec::new();

        for (id, entry) in manager.jobs.iter_mut() {
            match entry.receiver.try_recv() {
                Ok(JobMessage::Completed(result)) => {
                    if let Some(on_complete) = entry.on_complete.take() {
                        completions.push((on_complete, result));
                    }
                    finished_ids.push(id.clone());
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ok(JobMessage::Panicked(message)) => {
                    panic_info = Some((id.clone(), message));
                    finished_ids.push(id.clone());
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic_info = Some((
                        id.clone(),
                        "background thread disconnected without a panic payload".to_owned(),
                    ));
                    finished_ids.push(id.clone());
                    break;
                }
            }
        }

        for id in finished_ids {
            manager.jobs.remove(&id);
        }
    });

    if let Some((job_id, message)) = panic_info {
        panic!("Job '{}' panicked in the background: {}", job_id, message);
    }

    for (on_complete, result) in completions {
        on_complete(result);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::math::Dimensions;
    use crate::Ply;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    fn clear_jobs() {
        JOB_MANAGER.with(|manager| manager.borrow_mut().jobs.clear());
    }

    #[test]
    fn spawn_rejects_duplicate_id() {
        clear_jobs();

        let first = spawn("dup-job", || async { 1u32 }, |_| {});
        assert!(first.is_ok());

        let second = spawn("dup-job", || async { 2u32 }, |_| {});
        assert!(second.is_err());

        clear_jobs();
    }

    #[test]
    fn callback_runs_during_begin() {
        clear_jobs();

        let result = Rc::new(RefCell::new(None::<u32>));
        let result_for_callback = Rc::clone(&result);

        let spawn_result = spawn(
            "complete-job",
            || async { 42u32 },
            move |value| {
                *result_for_callback.borrow_mut() = Some(value);
            },
        );
        assert!(spawn_result.is_ok());

        let mut ply = Ply::<()>::new_headless(Dimensions::new(32.0, 32.0));
        for _ in 0..100 {
            let _ = ply.begin();
            if result.borrow().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }

        assert_eq!(*result.borrow(), Some(42));
        assert!(!running("complete-job"));

        clear_jobs();
    }

    #[test]
    fn background_panic_surfaces_original_message() {
        clear_jobs();

        let spawn_result = spawn(
            "panic-job",
            || async {
                panic!("boom from worker");
            },
            |_v: ()| {},
        );
        assert!(spawn_result.is_ok());

        let mut ply = Ply::<()>::new_headless(Dimensions::new(32.0, 32.0));
        let mut panic_message = None;

        for _ in 0..100 {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = ply.begin();
            }));

            if let Err(payload) = result {
                panic_message = Some(panic_payload_to_string(payload));
                break;
            }

            std::thread::sleep(Duration::from_millis(1));
        }

        let panic_message = panic_message.expect("expected panic from background job");
        assert!(panic_message.contains("panic-job"));
        assert!(panic_message.contains("boom from worker"));

        clear_jobs();
    }
}
