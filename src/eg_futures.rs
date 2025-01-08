#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct TimerFuture {
    start: Option<Instant>,
    time: Duration,
    timeout_ms: u64
}

impl TimerFuture {
    pub fn new (timeout_ms: u64) -> Self {
        Self {
            start: None,
            timeout_ms,
            time: Duration::from_millis(timeout_ms)
        }
    }
}

impl Future for TimerFuture {
    type Output = u64;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.start {
            None => {
                self.start = Some(Instant::now());
            },
            Some(x) => if x.elapsed() >= self.time {
                return Poll::Ready(self.timeout_ms);
            }
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub fn sleep (ms: u64) -> TimerFuture {
    TimerFuture::new(ms)
}

pub async fn slow_future (n: u64) -> u64 {
    let mut res = 0;
    std::thread::sleep(Duration::from_millis(n));
    for i in 0..n {
        res += i;
    }
    res
}