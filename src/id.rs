#[derive(Default)]
pub struct IdGenerator {
    next: usize,
    current_generation: usize
}

impl IdGenerator {
    pub const fn next (&mut self) -> Id {
        let ret = Id {
            index: self.next,
            generation: self.current_generation
        };

        #[allow(clippy::single_match_else)]
        match self.next.checked_add(1) {
            Some(new) => {
                self.next = new;
            },
            None => {
                self.next = 0;
                self.current_generation += 1;
            }
        }
        
        ret
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Id {
    index: usize,
    generation: usize,
}