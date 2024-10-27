use std::future::Future;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::pin::Pin;
use std::task::{RawWaker, Waker, Context, Poll};
use std::time::{Instant, Duration};

use waker::{WakerData, VTABLE};

mod waker;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub struct Executor {
    tasks_to_poll: Vec<Option<BoxedFuture<u32>>>,
    tasks_receiver: Receiver<usize>, 
    tasks_sender: Sender<usize>,
}

impl Default for Executor {
    fn default () -> Self {
        let (tasks_sender, tasks_receiver) = channel();
        Self {
            tasks_to_poll: Vec::new(),
            tasks_sender,
            tasks_receiver
        }
    }
}

impl Executor {
    pub fn block_on(mut self, future: BoxedFuture<u32>) {
        self.tasks_to_poll.push(Some(future));
        self.tasks_sender.send(0).unwrap();

        'main: loop {
            for index in self.tasks_receiver.try_iter() {
                if index >= self.tasks_to_poll.len() {
                    panic!("index out of bounds");
                }

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

                if let Some(task) = &mut self.tasks_to_poll[index] {
                    match task.as_mut().poll(&mut cx) {
                        Poll::Ready(res) => {
                            println!("[executor] Received Result: {res}");
                            self.tasks_to_poll[index] = None;

                            if index == 0 {
                                break 'main;
                            }            
                        },
                        Poll::Pending => {}
                    }
                }


            }
        }
    }
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
        println!("[timer {}] polling", self.timeout_ms);
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

struct LessEfficientTimer {
    start: Option<Instant>,
    time: Duration,
    timeout_ms: u32
}

impl LessEfficientTimer {
    pub fn new (timeout_ms: u32) -> Self {
        Self {
            start: None,
            timeout_ms,
            time: Duration::from_millis(timeout_ms as u64)
        }
    }
}

impl Future for LessEfficientTimer {
    type Output = u32;

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


fn main() {
    Executor::default().block_on(Box::pin(async move {
        println!("[main] Creating futures");

        let one = LessEfficientTimer::new(250);
        let two = LessEfficientTimer::new(500);
        let three = LessEfficientTimer::new(750);

        println!("[main] Created all");

        let t = Instant::now();
        let res = futures::join!(one, two, three);
        let elapsed = t.elapsed();
        println!("[main] Got {res:?} in {elapsed:?}");

        0
    }));
}
