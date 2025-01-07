use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::task::{Context, Poll, Waker};
use std::thread::JoinHandle;
use crate::BoxedFuture;

pub struct TaskRunner {
    handle: JoinHandle<()>,
    task_sender: Sender<(BoxedFuture<u32>, Waker)>,
    is_running: Arc<AtomicBool>,
    needs_to_stop: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Poll<u32>>>>
}

impl TaskRunner {
    pub fn new () -> Self {
        let is_running = Arc::new(AtomicBool::new(false));
        let needs_to_stop = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(None));
        let (task_sender, task_receiver) = channel::<(BoxedFuture<u32>, Waker)>();
        
        
        let thread_is_running = is_running.clone();
        let thread_stop = needs_to_stop.clone();
        let thread_result = result.clone();
        
        let handle = std::thread::spawn(move || {
            loop {
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                
                for (mut task, waker) in task_receiver.try_iter() {
                    thread_is_running.store(true, Ordering::SeqCst);
                    
                    let mut cx = Context::from_waker(&waker);
                    let res = task.as_mut().poll(&mut cx);
                    *thread_result.lock().unwrap() = Some(res);

                    thread_is_running.store(false, Ordering::SeqCst);
                }
            }
        });
        
        Self {
            handle,
            is_running,
            needs_to_stop,
            task_sender,
            result
        }
    }
    
    pub fn is_running (&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
    
    pub fn take_result (&mut self) -> Option<Poll<u32>> {
        self.result.lock().unwrap().take()
    }
    
    pub fn send_task (&self, fut: BoxedFuture<u32>, waker: Waker) {
        self.task_sender.send((fut, waker)).unwrap();
    }
    
    pub fn stop (self) {
        self.needs_to_stop.store(true, Ordering::SeqCst);
        self.handle.join().unwrap();
    }
}

pub struct Pool {
    runners: Vec<(TaskRunner, Option<usize>)>,
}

impl Pool {
    pub fn new<const N: usize> () -> Self {
        let mut v = Vec::with_capacity(N);
        for _ in 0..N {
            v.push((TaskRunner::new(), None));
        }
        Self {
            runners: v,
        }
    }
    
    pub fn run_future(&mut self, f: BoxedFuture<u32>, waker: Waker, index: usize) {
        'outer: loop {
            for (runner, task_index) in self.runners.iter_mut() {
                if !runner.is_running() {
                    runner.send_task(f, waker);
                    *task_index = Some(index);
                    
                    
                    break 'outer;
                }
            }
        }
    }
    
    pub fn collect_results (&mut self) -> Vec<(Poll<u32>, usize)> {
        self.runners.iter_mut().filter_map(|(runner, task_index)| {
            if let Some(index) = task_index {
                if let Some(res) = runner.take_result() {
                    let owned_index = *index;
                    Some((res, owned_index))
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    }
}