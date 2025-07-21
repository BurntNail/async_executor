use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct AtomicIdGenerator {
    next: AtomicUsize
}

impl AtomicIdGenerator {
    pub fn next (&self) -> Id {
        Id {
            index: self.next.fetch_add(1, Ordering::SeqCst)
        }
    }
}




#[derive(Default)]
pub struct IdGenerator {
    next: usize,
}

impl Iterator for IdGenerator {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.checked_add(1).map(|new| {
            let ret = Id { index: self.next };
            self.next = new;
            ret
        })
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Id {
    index: usize,
}

impl From<Id> for usize {
    fn from(value: Id) -> Self {
        value.index
    }
}