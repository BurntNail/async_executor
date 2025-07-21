use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::sync::mpsc::{channel, Sender};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

struct TimerThread {
    requests_tx: Sender<WakerAndEnd>,
}

struct WakerAndEnd {
    waker: Waker,
    end: Instant
}

impl WakerAndEnd {
    pub fn is_finished (&self) -> bool {
        self.end.checked_duration_since(Instant::now()).is_none()
    }
}

impl Eq for WakerAndEnd {}

impl PartialEq for WakerAndEnd {
    fn eq(&self, other: &Self) -> bool {
        self.end.eq(&other.end) && self.waker.will_wake(&other.waker)
    }
}

impl PartialOrd<Self> for WakerAndEnd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let s = Reverse(self.end);
        let o = Reverse(other.end);

        s.partial_cmp(&o)
    }
}

impl Ord for WakerAndEnd {
    fn cmp(&self, other: &Self) -> Ordering {
        let s = Reverse(self.end);
        let o = Reverse(other.end);

        s.cmp(&o)
    }
}

impl TimerThread {
    pub fn get () -> &'static Self {
        static INSTANCE: LazyLock<TimerThread> = LazyLock::new(|| {
            let (requests_tx, requests_rx) = channel();
            std::thread::Builder::new()
                .name("timer_thread".into())
                .spawn(move || {
                    let mut to_check = BinaryHeap::new();

                    loop {
                        to_check.extend(requests_rx.try_iter());
                        
                        let top_element_finished = to_check.peek().is_some_and(|wae: &WakerAndEnd| wae.is_finished());
                        if top_element_finished {
                            let top_element = to_check.pop().unwrap();
                            top_element.waker.wake();
                        } else {
                            std::thread::yield_now();
                        }
                    }
                })
                .expect("unable to spawn timer_thread");

            TimerThread {
                requests_tx,
            }
        });

        &INSTANCE
    }
}

enum TimerFutureState {
    Timer(Instant),
    Waiting(Instant),
    Done
}

pub struct TimerFuture {
    timeout: Duration,
    state: TimerFutureState
}

impl TimerFuture {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            state: TimerFutureState::Timer(Instant::now() + timeout)
        }
    }
}

impl Future for TimerFuture {
    type Output = Duration;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let thread = TimerThread::get();
        match std::mem::replace(&mut self.state, TimerFutureState::Done) {
            TimerFutureState::Timer(end) => {
                let wae = WakerAndEnd {
                    waker: cx.waker().clone(),
                    end
                };
                let _ = thread.requests_tx.send(wae);
                self.state = TimerFutureState::Waiting(end);
                Poll::Pending
            }
            TimerFutureState::Waiting(end) => {
                if end.checked_duration_since(Instant::now()).is_none() {
                    Poll::Ready(self.timeout)
                } else {
                    self.state = TimerFutureState::Waiting(end);
                    Poll::Pending
                }
            }
            TimerFutureState::Done => {
                panic!("tried to poll timer future after completion")
            }
        }
    }
}

pub async fn sleep_millis(ms: u64) -> u128 {
    TimerFuture::new(Duration::from_millis(ms)).await.as_millis()
}
pub async fn sleep_micros (ms: u64) -> u128 {
    TimerFuture::new(Duration::from_micros(ms)).await.as_micros()
}
