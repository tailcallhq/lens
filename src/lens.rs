use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Modify, Select, View};

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum Lens {
    Field(String),
    Index(usize),
    Compose(Box<Lens>, Box<Lens>),
    ForEach,
    #[default]
    Empty,
}

impl Lens {
    pub fn get_mut<'a>(&'a self, value: &'a mut Value) -> Option<Modify<'a>> {
        match self {
            Lens::Field(field) => value
                .as_object_mut()
                .and_then(|obj| obj.get_mut(field))
                .map(Modify::BorrowMut),
            Lens::Index(index) => value
                .as_array_mut()
                .and_then(|arr| arr.get_mut(*index))
                .map(Modify::BorrowMut),
            Lens::Compose(first, second) => {
                if let Some(inner) = first.get_mut(value) {
                    inner.get_mut(second.as_ref())
                } else {
                    None
                }
            }
            Lens::ForEach => value
                .as_array_mut()
                .map(|arr| Modify::BorrowVec(arr.iter_mut().map(Modify::BorrowMut).collect())),
            Lens::Empty => Some(Modify::BorrowMut(value)),
        }
    }

    pub fn get<'a>(&'a self, value: &'a Value) -> Option<View<'a>> {
        match self {
            Lens::Field(field) => value
                .as_object()
                .and_then(|obj| obj.get(field))
                .map(View::Borrow),
            Lens::Index(index) => value
                .as_array()
                .and_then(|arr| arr.get(*index))
                .map(View::Borrow),
            Lens::Compose(first, second) => {
                first.get(value).and_then(|view| view.get(second.as_ref()))
            }
            Lens::ForEach => value
                .as_array()
                .map(|arr| View::BorrowVec(arr.iter().map(View::Borrow).collect())),
            Lens::Empty => Some(View::Borrow(value)),
        }
    }

    pub fn set(&self, source: &mut Value, target: Value) {
        dbg!(self);
        match self {
            Lens::Field(field) => {
                if let Some(obj) = source.as_object_mut() {
                    obj.insert(field.clone(), target);
                }
            }
            Lens::Index(index) => {
                if let Some(arr) = source.as_array_mut() {
                    if *index < arr.len() {
                        arr[*index] = target;
                    }
                }
            }
            Lens::Compose(first, second) => {
                if let Some(modify) = first.get_mut(source) {
                    modify.set(second, target);
                }
            }
            Lens::ForEach => {
                if let Some(arr) = source.as_array_mut() {
                    arr.iter_mut().for_each(|source| {
                        *source = target.clone();
                    });
                }
            }
            Lens::Empty => {}
        }
    }

    pub fn select<I: Select>(self, item: I) -> Self {
        item.pipe(self)
    }

    pub fn new<I: Select>(item: I) -> Self {
        item.pipe(Lens::Empty)
    }

    pub fn foreach() -> Self {
        Lens::ForEach
    }

    pub fn each(self) -> Self {
        Lens::ForEach.pipe(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_field() {
        let mut value = json!({"a": 1});
        let lens = Lens::new("a");
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&json!(1)));

        lens.set(&mut value, json!(2));
        assert_eq!(value, json!({"a": 2}));
    }

    #[test]
    fn test_index() {
        let mut value = json!([1, 2, 3]);
        let lens = Lens::new(1);
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&json!(2)));

        lens.set(&mut value, json!(4));
        assert_eq!(value, json!([1, 4, 3]));
    }

    #[test]
    fn test_compose() {
        let mut value = json!({"a": [1, 2, 3]});
        let lens = Lens::new("a").select(1);
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&json!(2)));

        lens.set(&mut value, json!(4));
        assert_eq!(value, json!({"a": [1, 4, 3]}));
    }

    #[test]
    fn test_for_each() {
        let mut value = json!([{"a": 1}, {"a": 2}, {"a": 3}]);

        let lens = Lens::foreach().select("a");
        let view = lens.get(&value).unwrap();
        assert_eq!(
            view,
            View::BorrowVec(vec![
                View::Borrow(&json!(1)),
                View::Borrow(&json!(2)),
                View::Borrow(&json!(3))
            ])
        );

        lens.set(&mut value, json!(4));
        assert_eq!(value, json!([{"a": 4}, {"a": 4}, {"a": 4}]));
    }
    #[test]
    fn test_deeply_nested() {
        let mut value = json!({
            "a": {
                "b": {
                    "c": [1, 2, {"d": 3}]
                }
            }
        });

        let lens = Lens::default()
            .select("a")
            .select("b")
            .select("c")
            .select(2)
            .select("d");
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&json!(3)));

        lens.set(&mut value, json!(4));
        assert_eq!(
            value,
            json!({
                "a": {
                    "b": {
                        "c": [1, 2, {"d": 4}]
                    }
                }
            })
        );
    }

    #[test]
    fn test_flatten_structure() {
        let mut value = json!({
            "a": {
                "b": {
                    "c": [1, 2, {"d": 3}]
                }
            }
        });

        let lens = Lens::default().select("a").select("b").select("c").each();
        let view = lens.get(&value).unwrap();
        assert_eq!(
            view,
            View::BorrowVec(vec![
                View::Borrow(&json!(1)),
                View::Borrow(&json!(2)),
                View::Borrow(&json!({"d": 3}))
            ])
        );

        lens.set(&mut value, json!(4));
        assert_eq!(
            value,
            json!({
                "a": {
                    "b": {
                        "c": [4, 4, 4]
                    }
                }
            })
        );
    }
}
