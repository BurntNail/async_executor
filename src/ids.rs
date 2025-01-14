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
