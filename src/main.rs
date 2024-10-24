mod waker;

use crate::waker::{WakerData, VTABLE};
use std::{
    future::Future,
    sync::mpsc::{channel, Sender},
    task::{Context, Poll, RawWaker, Waker}, time::{Duration, Instant},
};
use std::{pin::Pin, sync::mpsc::Receiver};

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

#[derive(Debug)]
pub enum WakeResult {
    ByRef,
    Owned,
}

pub struct Executor {
    tasks_to_poll: Vec<Option<BoxedFuture<u32>>>,
    tasks_receiver: Receiver<(usize, WakeResult)>,
    tasks_sender: Sender<(usize, WakeResult)>,
}

impl Default for Executor {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tasks_to_poll: vec![],
            tasks_sender: tx,
            tasks_receiver: rx,
        }
    }
}

impl Executor {
    pub fn block_on(mut self, future: BoxedFuture<u32>) {
        self.tasks_to_poll.push(Some(future));
        self.tasks_sender
            .send((0, WakeResult::ByRef))
            .expect("unable to do things");

        'outer: loop {
            for (index, wr) in self.tasks_receiver.try_iter() {
                if index >= self.tasks_to_poll.len() {
                    panic!("index out of bounds");
                }
                // println!("[executor] polling {index} w/ {wr:?}");

                let waker_data = WakerData {
                    id: index,
                    tasks_sender: self.tasks_sender.clone(),
                };
                let boxed_waker_data = Box::new(waker_data);
                let raw_waker_data = Box::into_raw(boxed_waker_data);

                let raw_waker =
                    RawWaker::new(raw_waker_data as *const WakerData as *const (), &VTABLE);
                let waker = unsafe { Waker::from_raw(raw_waker) };
                let mut cx = Context::from_waker(&waker);

                let res = match wr {
                    WakeResult::ByRef => {
                        if let Some(task) = &mut self.tasks_to_poll[index] {
                            match task.as_mut().poll(&mut cx) {
                                Poll::Ready(res) => res,
                                Poll::Pending => continue
                            }
                        } else {
                            eprintln!("[executor] Asked to poll non-existent task {index}");
                            continue;
                        }
                    }
                    WakeResult::Owned => {
                        if let Some(mut task) = std::mem::take(&mut self.tasks_to_poll[index]) {
                            match task.as_mut().poll(&mut cx) {
                                Poll::Ready(res) => res,
                                Poll::Pending => continue,
                            }
                        } else {
                            eprintln!("[executor] Asked to poll non-existent task {index}");
                            continue;
                        }
                    }
                };

                println!("[executor] Received Result: {res}");
                if index == 0 {
                    break 'outer;
                }
            }
        }
    }
}

fn main() {
    Executor::default().block_on(Box::pin(async move {
        println!("Starting futures");

        let fa = TimerFuture::new(500);
        let fb = TimerFuture::new(500);
        let fc = TimerFuture::new(500);

        println!("Started all");
    
        let t = Instant::now();
        let res = futures::join!(fa, fb, fc);
        let elapsed = t.elapsed();
        println!("Got {res:?} in {elapsed:?}");
        
        res.0
    }));
}

struct TimerFuture {
    slept: bool,
    timeout_ms: u32
}

impl TimerFuture {
    pub fn new (timeout_ms: u32) -> Self {
        Self {timeout_ms, slept: false}
    }
}

impl Future for TimerFuture {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.slept {
            let waker = cx.waker().clone();
            let time = self.timeout_ms as u64;
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(time));
                waker.wake();
            });
            self.slept = true;

            Poll::Pending
        } else {
            Poll::Ready(self.timeout_ms)
        }
    }
}
