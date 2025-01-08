use std::time::{Instant};
use crate::eg_futures::{slow_future, TimerFuture};
use crate::executor::Executor;

mod waker;
mod eg_futures;
mod task_runner;
mod ids;
mod executor;

#[macro_export]
macro_rules! prt {
    ($($arg:tt)*) => {{
        if false {
            println!($($arg)*)
        }
    }};
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
    let instant = executor.run(create_task(0, 0)).unwrap();
    let id1 = executor.run(create_task(15, 1)).unwrap();
    let id2 = executor.run(create_task(5, 2)).unwrap();
    let id3 = executor.run(create_task(15, 3)).unwrap();
    let id4 = executor.run(create_task(20, 4)).unwrap();
    
    let fib = executor.run(slow_future(18)).unwrap();
    
    let instant: Option<u64> = executor.take_result(&instant).into();
    println!("[main] instant result actually instant? {instant:?}");

    println!("[main] created all tasks, joining executor");
    let mut executor = executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");

    let res1: u64 = executor.take_result(&id1).unwrap();
    let res2: u64 = executor.take_result(&id2).unwrap();
    let res3: u64 = executor.take_result(&id3).unwrap();
    let res4: u64 = executor.take_result(&id4).unwrap();
    let fib: u64 = executor.take_result(&fib).unwrap();

    assert_eq!(res1, 15);
    assert_eq!(res2, 5);
    assert_eq!(res3, 15);
    assert_eq!(res4, 20);

    println!("[main] slow calc is {fib:?}");
}
