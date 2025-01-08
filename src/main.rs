use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::mpsc::{Sender, channel, Receiver};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{RawWaker, Waker, Context, Poll};
use std::thread::JoinHandle;
use std::time::{Instant, Duration};
use waker::{WakerData, VTABLE};

mod waker;

pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type ErasedFuture = BoxedFuture<Box<dyn Any + Send>>;

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

#[macro_export]
macro_rules! prt {
    ($($arg:tt)*) => {{
        if false {
            println!($($arg)*)
        }
    }};
}

pub struct Running {
    can_finish: Arc<AtomicBool>,
    running_thread: JoinHandle<()>,
    id_generator: IdGenerator
}
pub struct Finished;

pub struct Executor<Stage> {
    new_tasks_sender: Sender<(Id, ErasedFuture)>,
    results_rx: Receiver<(Id, Box<dyn Any + Send>)>,
    results_cache: HashMap<Id, Box<dyn Any + Send>>,
    stage_details: Stage,
}

impl Executor<Running> {
    pub fn start() -> Self {
        let (new_tasks_sender, new_tasks_receiver) = channel();
        let (results_sender, results_rx) = channel();
        let can_finish = Arc::new(AtomicBool::new(false));

        let thread_can_finish = can_finish.clone();
        let running_thread = std::thread::spawn(move || {
            let (tasks_sender, tasks_receiver) = channel();
            let mut tasks_to_poll: HashMap<Id, ErasedFuture> = HashMap::new();
            loop {
                for (index, future) in new_tasks_receiver.try_iter() {
                    tasks_to_poll.insert(index, future);
                    println!("[executor] adding new task @ {index:?}");
                    tasks_sender.send(index).unwrap();
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

                            if let Poll::Ready(res) = occ.get_mut().as_mut().poll(&mut cx) {
                                println!("[executor] Finished {index:?}");
                                let _= results_sender.send((index, res));
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
            results_rx,
            results_cache: HashMap::new(),
            stage_details: Running {
                id_generator: IdGenerator::default(),
                can_finish,
                running_thread,
            }
        }
    }

    pub fn join (self) -> Executor<Finished> {
        self.stage_details.can_finish.store(true, Ordering::SeqCst);
        self.stage_details.running_thread.join().unwrap();
        
        Executor {
            new_tasks_sender: self.new_tasks_sender,
            results_rx: self.results_rx,
            results_cache: self.results_cache,
            stage_details: Finished
        }
    }

    pub fn run<F: Future + Send + 'static> (&mut self, f: F) -> Option<Id>
    where F::Output: Send
    {
        let id = self.stage_details.id_generator.next();
        if let Some(id) = id {
            self.new_tasks_sender.send((id, Box::pin(async {
                let res = f.await;
                let boxed: Box<dyn Any + Send> = Box::new(res);
                boxed
            }))).unwrap();
        }
        id
    }
}

impl<Stage> Executor<Stage> {
    pub fn take_result<T: 'static> (&mut self, id: &Id) -> FutureResult<T> {
        self.results_cache.extend(self.results_rx.try_iter());
        match self.results_cache.remove(id) {
            None => FutureResult::NonExistent,
            Some(x) => match x.downcast() {
                Ok(t) => FutureResult::Expected(*t),
                Err(b) => FutureResult::Other(b),
            }
        }
    }
}

#[derive(Debug)]
pub enum FutureResult<T> {
    Expected(T),
    Other(Box<dyn Any>),
    NonExistent
}

impl<T> FutureResult<T> {
    pub fn unwrap (self) -> T {
        match self {
            Self::Expected(e) => e,
            Self::Other(_) => panic!("failed to unwrap FR as wrong type"),
            Self::NonExistent => panic!("failed to unwrap FR as `None`")
        }
    }
}

struct TimerFuture {
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


fn main() {
    let mut executor = Executor::start();
   
    let create_task = |time, local_id| async move {
        let fut = TimerFuture::new(time);
        println!("[task {local_id}] created future");

        let start = Instant::now();
        let res = fut.await;
        let el = start.elapsed();

        println!("[task {local_id}] awaited future in {el:?}");

        res
    };

    let start = Instant::now();
    let id1 = executor.run(create_task(150, 1)).unwrap();
    let id2 = executor.run(create_task(150, 2)).unwrap();
    let id3 = executor.run(create_task(50, 3)).unwrap();
    let id4 = executor.run(create_task(200, 4)).unwrap();
    
    std::thread::sleep(Duration::from_millis(75));
    
    // let fail: u32 = executor.take_result(&id4).unwrap();

    println!("[main] created all tasks, joining executor");
    let mut executor = executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");

    let res1: u64 = executor.take_result(&id1).unwrap();
    let res2: u64 = executor.take_result(&id2).unwrap();
    let res3: u64 = executor.take_result(&id3).unwrap();
    let res4: u64 = executor.take_result(&id4).unwrap();
    
    println!("[main] got results ({res1:?},{res2:?},{res3:?},{res4:?})");
}
