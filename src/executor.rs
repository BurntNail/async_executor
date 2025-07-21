use crate::executor::sealed::CanUseCannotImplement;
use crate::id::{Id, IdGenerator};
use task_runner::Pool;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

mod task_runner;
mod waker;

pub type Erased = Box<dyn Any + Send>;
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub trait ExecutorStage: CanUseCannotImplement {}
mod sealed {
    use crate::executor::{Finished, Running};

    pub trait CanUseCannotImplement {}
    impl CanUseCannotImplement for Running {}
    impl CanUseCannotImplement for Finished {}
}
impl<T: CanUseCannotImplement> ExecutorStage for T {}

pub struct Running {
    pool: Pool,
    id_generator: IdGenerator,
}
pub struct Finished;

pub struct Executor<Stage: ExecutorStage> {
    results_cache: HashMap<Id, Erased>,
    stage_details: Stage,
}

impl Executor<Running> {
    pub fn start(n_workers: usize) -> Self {
        Self {
            results_cache: HashMap::new(),
            stage_details: Running {
                pool: Pool::new(n_workers),
                id_generator: IdGenerator::default(),
            },
        }
    }

    pub fn run<F: Future + Send + 'static>(&mut self, f: F) -> Option<Id>
    where
        F::Output: Send,
    {
        let id = self.stage_details.id_generator.next();
        if let Some(id) = id {
            self.stage_details.pool.run_future(
                id,
                Box::pin(async {
                    let res = f.await;
                    let erased: Erased = Box::new(res);
                    erased
                }),
            );
        }
        id
    }

    pub fn take_result<T: 'static>(&mut self, id: Id) -> FutureResult<T> {
        self.results_cache
            .extend(self.stage_details.pool.collect_results());
        self.take_result_no_update(id)
    }

    pub fn join(mut self) -> Executor<Finished> {
        self.results_cache.extend(self.stage_details.pool.join());

        Executor {
            results_cache: self.results_cache,
            stage_details: Finished,
        }
    }
}

impl Executor<Finished> {
    pub fn take_result<T: 'static>(&mut self, id: Id) -> FutureResult<T> {
        self.take_result_no_update(id)
    }
}

impl<S: ExecutorStage> Executor<S> {
    fn take_result_no_update<T: 'static>(&mut self, id: Id) -> FutureResult<T> {
        self.results_cache.remove(&id).map_or_else(|| FutureResult::NonExistent, |x| match x.downcast() {
            Ok(t) => FutureResult::Expected(*t),
            Err(b) => FutureResult::Other(b),
        })
    }
}

#[derive(Debug)]
pub enum FutureResult<T> {
    Expected(T),
    #[allow(dead_code)]
    Other(Box<dyn Any>),
    NonExistent,
}

impl<T> FutureResult<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Expected(e) => e,
            Self::Other(_) => panic!("failed to unwrap FutureResult as found wrong type"),
            Self::NonExistent => panic!("failed to unwrap FutureResult as `None`"),
        }
    }
}

impl<T> From<FutureResult<T>> for Option<T> {
    fn from(value: FutureResult<T>) -> Self {
        match value {
            FutureResult::Expected(x) => Some(x),
            _ => None,
        }
    }
}
