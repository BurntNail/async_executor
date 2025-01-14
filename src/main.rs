#![allow(dead_code)]

use crate::eg_futures::{blocking_slow_future, TimerFuture};
use crate::executor::Executor;
use std::time::Instant;

mod eg_futures;
mod executor;
mod ids;
mod task_runner;
mod waker;

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
        println!("[task {local_id}] started future, awaiting timerfuture");

        let start = Instant::now();
        let res = fut.await;
        let el = start.elapsed();

        println!("[task {local_id}] awaited timerfuture in {el:?}");

        res
    };

    let start = Instant::now();
    let id1 = executor.run(create_task(150, 1)).unwrap();
    let id2 = executor.run(create_task(50, 2)).unwrap();
    let id3 = executor.run(create_task(150, 3)).unwrap();
    let id4 = executor.run(create_task(200, 4)).unwrap();

    let fib = executor.run(blocking_slow_future(18)).unwrap();

    println!("[main] created all tasks, joining executor");
    let mut executor = executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");

    let res1: u64 = executor.take_result(&id1).unwrap();
    let res2: u64 = executor.take_result(&id2).unwrap();
    let res3: u64 = executor.take_result(&id3).unwrap();
    let res4: u64 = executor.take_result(&id4).unwrap();
    let fib: u64 = executor.take_result(&fib).unwrap();

    assert_eq!(res1, 150);
    assert_eq!(res2, 50);
    assert_eq!(res3, 150);
    assert_eq!(res4, 200);

    println!("[main] slow calc is {fib:?}");
}
