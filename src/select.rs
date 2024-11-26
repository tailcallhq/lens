use crate::Lens;

pub trait Select {
    fn pipe(self, lens: Lens) -> Lens;
}

impl Select for Lens {
    fn pipe(self, other: Lens) -> Lens {
        Lens::Compose(Box::new(other), Box::new(self))
    }
}

impl Select for &str {
    fn pipe(self, lens: Lens) -> Lens {
        lens.select(self.to_string())
    }
}

impl Select for String {
    fn pipe(self, lens: Lens) -> Lens {
        lens.select(Lens::Field(self))
    }
}

impl Select for usize {
    fn pipe(self, lens: Lens) -> Lens {
        lens.select(Lens::Index(self))
    }
}
