use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::adapters::io_worker::{IoOutcome, IoReqOptions, IoThread};
use crate::id::Id;

mod io_worker;
pub mod file;
pub mod net;
pub mod fs;


enum SimpleThreadFutureState {
    NotYetStarted(IoReqOptions),
    Waiting(Id),
    Done
}

struct SimpleThreadFuture<F>
{
    pub state: SimpleThreadFutureState,
    pub check_outcome: Box<F>
}

impl<T, F> SimpleThreadFuture<F>
where F: Fn(IoOutcome) -> Option<T> {
    pub fn new (io_req_options: IoReqOptions, check_outcome: F) -> Self {
        Self {
            state: SimpleThreadFutureState::NotYetStarted(io_req_options),
            check_outcome: Box::new(check_outcome)
        }
    }
}


impl<T, F> Future for SimpleThreadFuture<F>
where F: Fn(IoOutcome) -> Option<T>
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let thread = IoThread::get();
        match std::mem::replace(&mut self.state, SimpleThreadFutureState::Done) {
            SimpleThreadFutureState::NotYetStarted(request_options) => {
                let task_id = thread.send_request(request_options, cx.waker().clone());
                self.state = SimpleThreadFutureState::Waiting(task_id);
                Poll::Pending
            }
            SimpleThreadFutureState::Waiting(task_id) => {
                match thread.try_recv_result(task_id) {
                    None => {
                        self.state = SimpleThreadFutureState::Waiting(task_id);
                        Poll::Pending
                    }
                    Some(res) => {
                        (self.check_outcome)(res)
                            .map_or_else(
                                || panic!("found incorrect type from io thread"),
                                Poll::Ready
                            )
                    }
                }
            }
            SimpleThreadFutureState::Done => {
                panic!("tried to poll io thread future after completion")
            }
        }
    }
}