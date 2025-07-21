#![allow(dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use crate::timer_future::sleep;
use crate::executor::Executor;
use std::time::{Duration, Instant};
use crate::adapters::file::File;
use adapters::net::fully_read_from_socket;

mod timer_future;
mod executor;
mod id;
mod adapters;

#[macro_export]
macro_rules! prt {
    ($($arg:tt)*) => {{
        if false {
            println!($($arg)*)
        }
    }};
}

fn timer_bits () {
    #[allow(clippy::unused_async)]
    async fn blocking_slow_future(n: u64) -> u64 {
        let mut res = 0;
        for i in 0..n {
            std::thread::sleep(Duration::from_millis(100));
            res += i;
        }
        res
    }

    async fn check_string (input: impl AsRef<str> + Send) -> String {
        let input = input.as_ref();
        let mut output = String::with_capacity(input.len());

        for ch in input.chars() {
            sleep(ch as u64).await;
            output.push(ch);
        }

        output
    }

    let mut executor = Executor::start(16);

    let create_task = |time, local_id| async move {
        let fut = sleep(time);
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

    let st = executor.run(check_string("Hello, World!")).unwrap();
    let fib = executor.run(blocking_slow_future(18)).unwrap();

    println!("[main] created all tasks, joining executor");
    let mut executor = executor.join();
    let el = start.elapsed();
    println!("[main] joined, took {el:?}");

    let res1: u64 = executor.take_result(id1).unwrap();
    let res2: u64 = executor.take_result(id2).unwrap();
    let res3: u64 = executor.take_result(id3).unwrap();
    let res4: u64 = executor.take_result(id4).unwrap();

    let fib: u64 = executor.take_result(fib).unwrap();
    let st: String = executor.take_result(st).unwrap();

    assert_eq!(res1, 150);
    assert_eq!(res2, 50);
    assert_eq!(res3, 150);
    assert_eq!(res4, 200);

    println!("[main] slow calc is {fib:?}");
    println!("[main] checked string is {st:?}");
}

fn tcp_bits () {
    let mut executor = Executor::start(1);

    let printer_boi = |count, delay| async move {
        let start = Instant::now();
        for _ in 0..count {
            sleep(delay).await;
            println!("[main] waited again :), now @ {:?}", start.elapsed());
        }
    };

    let streamer = async move {
        let res = fully_read_from_socket("0.0.0.0:8080").await.unwrap();
        let stringed = String::from_utf8(res).unwrap();
        println!("[main] got output from TCP: {stringed:?}");
    };


    executor.run(printer_boi(1000, 100)).unwrap();
    executor.run(streamer).unwrap();

    executor.join();
}

fn file_bits () {
    let mut executor = Executor::start(1);

    let create_timer_task = |time, local_id| async move {
        let fut = sleep(time);
        println!("[task {local_id}] started future, awaiting timerfuture");

        let start = Instant::now();
        let res = fut.await;
        let el = start.elapsed();

        println!("[task {local_id}] awaited timerfuture in {el:?}");

        res
    };
    
    let timers = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        .map(|timer_len| create_timer_task(timer_len, timer_len));
    
    let file_open_and_read = async move {
        println!("[task file] opening file");
        let mut file = File::open("Cargo.toml").await.expect("unable to open file");
        println!("[task file] opened file, reading bytes");

        let mut contents = vec![];
        loop {
            let this_round = file.read(1).await.expect("error reading");
            if this_round.is_empty() {
                break;
            }
            
            contents.extend_from_slice(&this_round);
        }

        let file_contents_as_string = String::from_utf8(contents).expect("invalid UTF-8");
        println!("[task file] read {file_contents_as_string:?}");
    };


    for timer in timers {
        executor.run(timer).unwrap();
    }
    executor.run(file_open_and_read).unwrap();

    executor.join();
}

fn main() {
    file_bits();
}
