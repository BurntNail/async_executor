use crate::prt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct TimerFuture {
    start: Option<Instant>,
    time: Duration,
    timeout_ms: u64,
}

impl TimerFuture {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            start: None,
            timeout_ms,
            time: Duration::from_millis(timeout_ms),
        }
    }
}

impl Future for TimerFuture {
    type Output = u64;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        prt!("[timerfuture] polling");
        match self.start {
            None => {
                prt!("[timerfuture] first poll, setting start");
                self.start = Some(Instant::now());

                std::thread::spawn({
                    let cx = cx.waker().clone();
                    let time = self.time;

                    move || {
                        std::thread::sleep(time);
                        cx.wake();

                        prt!("[timerfuture] asked for wake");
                    }
                });
            }
            Some(x) => {
                if x.elapsed() >= self.time {
                    return Poll::Ready(self.timeout_ms);
                }
            }
        }

        prt!("[timerfuture] not yet finished");

        Poll::Pending
    }
}

pub fn sleep(ms: u64) -> TimerFuture {
    TimerFuture::new(ms)
}

pub async fn blocking_slow_future(n: u64) -> u64 {
    let mut res = 0;
    for i in 0..n {
        std::thread::sleep(Duration::from_millis(100));
        // sleep(10).await;
        res += i;
    }
    res
}
