use crate::executor::{BoxedFuture, Erased};
use crate::ids::Id;
use crate::waker::{WakerData, VTABLE};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, Waker};
use std::thread::JoinHandle;
use crate::prt;

pub struct TaskRunner {
    handle: JoinHandle<()>,
    task_sender: Sender<(Id, BoxedFuture<Erased>)>,
    current_tasks: Arc<AtomicUsize>,
    needs_to_stop: Arc<AtomicBool>,
    result_receiver: Receiver<(Id, Erased)>,
}

impl TaskRunner {
    pub fn new() -> Self {
        let current_tasks = Arc::new(AtomicUsize::new(0));
        let needs_to_stop = Arc::new(AtomicBool::new(false));
        let (task_sender, task_receiver) = channel::<(Id, BoxedFuture<Erased>)>();
        let (result_sender, result_receiver) = channel();

        let thread_current_tasks = current_tasks.clone();
        let thread_stop = needs_to_stop.clone();

        let handle = std::thread::spawn(move || {
            let (poll_sender, poll_receiver) = channel::<Id>();

            let mut to_poll = HashMap::new();
            loop {
                for (id, fut) in task_receiver.try_iter() {
                    to_poll.insert(id, fut);
                    poll_sender.send(id).unwrap();
                }

                if thread_stop.load(Ordering::SeqCst) && to_poll.is_empty() {
                    break;
                }

                for id in poll_receiver.try_iter() {
                    match to_poll.entry(id) {
                        Entry::Occupied(mut occ) => {
                            let waker_data = Box::new(WakerData::new(poll_sender.clone(), id));
                            let waker_data = Box::into_raw(waker_data);

                            let raw_waker = RawWaker::new(waker_data as *const _, &VTABLE);
                            let waker = unsafe { Waker::from_raw(raw_waker) };
                            let mut cx = Context::from_waker(&waker);

                            if let Poll::Ready(res) = occ.get_mut().as_mut().poll(&mut cx) {
                                let _ = result_sender.send((id, res));
                                drop(occ.remove());
                                thread_current_tasks.fetch_sub(1, Ordering::Relaxed);
                            }
                        }
                        Entry::Vacant(_) => {
                            eprintln!("[task runner] tried to poll non-existent task");
                        }
                    }
                }
            }
        });

        Self { handle, task_sender, current_tasks, needs_to_stop, result_receiver }
    }

    pub fn current_number_of_tasks(&self) -> usize {
        self.current_tasks.load(Ordering::Relaxed)
    }

    pub fn take_results(&self) -> impl Iterator<Item = (Id, Erased)> + use<'_> {
        self.result_receiver.try_iter()
    }

    pub fn send_task(&self, id: Id, fut: BoxedFuture<Erased>) {
        self.current_tasks.fetch_add(1, Ordering::Relaxed);
        self.task_sender.send((id, fut)).unwrap();
    }

    pub fn join(self) -> impl Iterator<Item = (Id, Erased)> {
        self.needs_to_stop.store(true, Ordering::SeqCst);
        self.handle.join().unwrap();
        self.result_receiver.into_iter()
    }
}

pub struct Pool {
    runners: Vec<TaskRunner>,
}

impl Pool {
    pub fn new(n_workers: usize) -> Self {
        Self {
            runners: (0..n_workers).map(|_| TaskRunner::new()).collect(),
        }
    }

    pub fn run_future(&self, id: Id, f: BoxedFuture<Erased>) {
        let (runner_index, _) = self
            .runners
            .iter()
            .enumerate()
            .map(|(i, runner)| (i, runner.current_number_of_tasks()))
            .min_by_key(|(_, n)| *n)
            .unwrap();
        prt!("[pool] sending task to {runner_index}");
        self.runners[runner_index].send_task(id, f);
    }

    pub fn collect_results(&self) -> impl Iterator<Item = (Id, Erased)> + use<'_> {
        self.runners
            .iter()
            .flat_map(TaskRunner::take_results)
    }

    pub fn join(self) -> impl Iterator<Item = (Id, Erased)> {
        self.runners.into_iter().flat_map(TaskRunner::join)
    }
}
