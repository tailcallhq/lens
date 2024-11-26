use serde_json::Value;

use crate::Lens;

#[derive(Debug, PartialEq, Eq)]
pub enum Modify<'a> {
    BorrowMut(&'a mut Value),
    BorrowVec(Vec<Modify<'a>>),
}

impl<'a> Modify<'a> {
    pub fn get_mut(self, lens: &'a Lens) -> Option<Self> {
        match self {
            Modify::BorrowMut(value) => lens.get_mut(value),
            Modify::BorrowVec(vec) => Some(Modify::BorrowVec(
                vec.into_iter()
                    .filter_map(|value| value.get_mut(lens))
                    .collect(),
            )),
        }
    }

    pub fn set(self, lens: &Lens, new_value: Value) {
        match self {
            Modify::BorrowMut(value) => {
                lens.set(value, new_value);
            }
            Modify::BorrowVec(values) => {
                values
                    .into_iter()
                    .for_each(|value| value.set(lens, new_value.clone()));
            }
        }
    }
}
