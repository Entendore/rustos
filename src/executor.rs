use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::future::Future;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use spin::Mutex;

type Task = core::pin::Pin<Box<dyn Future<Output = ()> + Send>>;

static QUEUE: Mutex<VecDeque<Task>> = Mutex::new(VecDeque::new());

pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    QUEUE.lock().push_back(Box::pin(future));
}

pub fn poll_all() {
    let mut queue = QUEUE.lock();
    let len = queue.len();
    for _ in 0..len {
        let task = match queue.pop_front() {
            Some(t) => t,
            None => break,
        };
        drop(queue);

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        
        if task.as_mut().poll(&mut cx).is_pending() {
            QUEUE.lock().push_back(task);
        }
    }
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
}

const VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(core::ptr::null(), &VTABLE),
    |_| {},
    |_| {},
    |_| {},
);
