#[derive(Default)]
pub struct IdGenerator {
    next: usize,
}

impl Iterator for IdGenerator {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next.checked_add(1) {
            Some(new) => {
                let ret = Some(Id { index: self.next });
                self.next = new;
                ret
            }
            None => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Id {
    index: usize,
}
