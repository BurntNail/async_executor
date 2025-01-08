use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use crate::eg_futures::{slow_future, TimerFuture};
use crate::task_runner::Pool;

mod waker;
mod eg_futures;
mod task_runner;

pub type Erased = Box<dyn Any + Send>;
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

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
    pool: Pool,
    id_generator: IdGenerator
}
pub struct Finished;

pub struct Executor<Stage> {
    results_cache: HashMap<Id, Erased>,
    stage_details: Stage,
}

impl Executor<Running> {
    pub fn start() -> Self {
        Executor {
            results_cache: HashMap::new(),
            stage_details: Running {
                pool: Pool::new::<16>(),
                id_generator: IdGenerator::default()
            }
        }
    }

    pub fn run<F: Future + Send + 'static> (&mut self, f: F) -> Option<Id>
    where F::Output: Send
    {
        let id = self.stage_details.id_generator.next();
        if let Some(id) = id {
            self.stage_details.pool.run_future(id, Box::pin(async {
                let res = f.await;
                let erased: Erased = Box::new(res);
                erased
            }));
        }
        id
    }

    pub fn take_result<T: 'static> (&mut self, id: &Id) -> FutureResult<T> {
        self.results_cache.extend(self.stage_details.pool.collect_results());
        match self.results_cache.remove(id) {
            None => FutureResult::NonExistent,
            Some(x) => match x.downcast() {
                Ok(t) => FutureResult::Expected(*t),
                Err(b) => FutureResult::Other(b),
            }
        }
    }

    pub fn join (mut self) -> Executor<Finished> {
        self.results_cache.extend(self.stage_details.pool.join());

        Executor {
            results_cache: self.results_cache,
            stage_details: Finished
        }
    }
}

impl Executor<Finished> {
    pub fn take_result<T: 'static> (&mut self, id: &Id) -> FutureResult<T> {
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
            Self::Other(_) => panic!("failed to unwrap FutureResult as found wrong type"),
            Self::NonExistent => panic!("failed to unwrap FutureResult as `None`")
        }
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
    
    let fib = executor.run(slow_future(175)).unwrap();
    
    // let fail: u32 = executor.take_result(&id4).unwrap();

    println!("[main] created all tasks, joining executor");
    let mut executor = executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");

    let res1: u64 = executor.take_result(&id1).unwrap();
    let res2: u64 = executor.take_result(&id2).unwrap();
    let res3: u64 = executor.take_result(&id3).unwrap();
    let res4: u64 = executor.take_result(&id4).unwrap();
    let fib: u64 = executor.take_result(&fib).unwrap();

    println!("[main] got results ({res1:?},{res2:?},{res3:?},{res4:?})");
    println!("[main] slow calc is {fib:?}");
}
