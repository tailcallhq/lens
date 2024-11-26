use serde_json::Value;

use crate::Lens;

#[derive(Debug, PartialEq, Eq)]
pub enum View<'a> {
    Borrow(&'a Value),
    BorrowVec(Vec<View<'a>>),
}

impl<'a> View<'a> {
    pub fn get(self, lens: &'a Lens) -> Option<Self> {
        match self {
            View::Borrow(value) => lens.get(value),
            View::BorrowVec(values) => Some(View::BorrowVec(
                values
                    .into_iter()
                    .filter_map(|value| value.get(lens))
                    .collect(),
            )),
        }
    }
}
