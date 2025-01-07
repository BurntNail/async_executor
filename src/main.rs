use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::Future;
use std::sync::mpsc::{Sender, channel};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{RawWaker, Waker, Context, Poll};
use std::thread::JoinHandle;
use std::time::{Instant, Duration};

use waker::{WakerData, VTABLE};

#[derive(Default)]
pub struct IdGenerator {
    next: usize
}

impl Iterator for IdGenerator {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == usize::MAX {
            None
        } else {
            let ret = Some(Id {
                index: self.next
            });
            self.next += 1;
            ret
        }
    }
}

#[derive(Copy, Clone, Default, Hash, Debug, Eq, PartialEq)]
pub struct Id {
    index: usize,
}

impl Id {
    pub fn next (&self) -> Self {
        Self {
            index: self.index + 1
        }
    }
}

mod waker;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type ErasedFuture = BoxedFuture<Box<dyn Any>>;


#[macro_export]
macro_rules! prt {
    ($($arg:tt)*) => {{
        if false {
            println!($($arg)*)
        }
    }};
}

pub struct Executor {
    new_tasks_sender: Sender<ErasedFuture>,
    can_finish: Arc<AtomicBool>,
    running_thread: JoinHandle<()>,
}

impl Executor {
    pub fn start() -> Executor {
        let (new_tasks_sender, new_tasks_receiver) = channel();
        let can_finish = Arc::new(AtomicBool::new(false));

        let thread_can_finish = can_finish.clone();
        let running_thread = std::thread::spawn(move || {
            let (tasks_sender, tasks_receiver) = channel();
            let mut tasks_to_poll: HashMap<Id, ErasedFuture> = HashMap::new();
            let mut id = IdGenerator::default();

            loop {
                for future in new_tasks_receiver.try_iter() {
                    if let Some(index) = id.next() {
                        tasks_to_poll.insert(index, future);
                        println!("[executor] adding new task @ {index:?}");
                        tasks_sender.send(index).unwrap();
                    }
                }

                for index in tasks_receiver.try_iter() {
                    match tasks_to_poll.entry(index) {
                        Entry::Occupied(mut occ) => {
                            prt!("[executor] polling {index:?}");

                            let waker_data = WakerData::new(tasks_sender.clone(), index);
                            let boxed_waker_data = Box::new(waker_data);
                            let raw_waker_data = Box::into_raw(boxed_waker_data);

                            let raw_waker =
                                RawWaker::new(raw_waker_data as *const WakerData as *const (), &VTABLE);
                            let waker = unsafe { Waker::from_raw(raw_waker) };
                            let mut cx = Context::from_waker(&waker);
                            
                            if let Poll::Ready(_res) = occ.get_mut().as_mut().poll(&mut cx) {
                                println!("[executor] Finished {index:?}");
                                occ.remove();
                            }
                        }
                        Entry::Vacant(_) => {
                            eprintln!("Tried to poll non-existent future index: {index:?}");
                        }
                    }
                }
                
                
                if thread_can_finish.load(Ordering::Relaxed) && tasks_to_poll.is_empty() {
                    break;
                }
            }
        });

        Executor {
            new_tasks_sender,
            can_finish,
            running_thread
        }
    }
    
    pub fn join (self) {
        self.can_finish.store(true, Ordering::SeqCst);
        self.running_thread.join().unwrap();
    }

    pub fn run<F: Future + Send + 'static> (&self, f: F) {
        self.new_tasks_sender.send(Box::pin(async {
            let res = f.await;
            let boxed: Box<dyn Any> = Box::new(res);
            boxed
        })).unwrap();
    }
}

struct TimerFuture {
    start: Option<Instant>,
    time: Duration,
    timeout_ms: u32
}

impl TimerFuture {
    pub fn new (timeout_ms: u32) -> Self {
        Self {
            start: None,
            timeout_ms,
            time: Duration::from_millis(timeout_ms as u64)
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.start {
            None => {
                self.start = Some(Instant::now());
            },
            Some(x) => if x.elapsed() >= self.time {
                return Poll::Ready(());
            }
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}


fn main() {
    let executor = Executor::start();
   
    let create_task = |time, id| async move {
        let fut = TimerFuture::new(time);
        println!("[task {id}] created future");

        let start = Instant::now();
        let res = fut.await;
        let el = start.elapsed();

        println!("[task {id}] awaited future, got {res:?} in {el:?}");

        String::from("yay")
    };

    let task_1 = create_task(150, 1);
    let task_2 = create_task(150, 2);
    let task_3 = create_task(50, 3);
    let task_4 = create_task(200, 4);
    
    let start = Instant::now();
    executor.run(task_1);
    executor.run(task_2);
    executor.run(task_3);
    executor.run(task_4);

    println!("[main] created all tasks, joining executor");
    executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");
}
