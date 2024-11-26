use serde_json::Value;

use crate::{Modify, Select, View};

#[derive(Default, Debug)]
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

    pub fn set(&self, value: &mut Value, new_value: Value) {
        dbg!(self);
        match self {
            Lens::Field(field) => {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(field.clone(), new_value);
                }
            }
            Lens::Index(index) => {
                if let Some(arr) = value.as_array_mut() {
                    if *index < arr.len() {
                        arr[*index] = new_value;
                    }
                }
            }
            Lens::Compose(first, second) => {
                dbg!(first);

                if let Some(modify) = first.get_mut(value) {
                    dbg!(&modify);    
                    modify.set(second, new_value);
                }
                dbg!("ok!");
            }
            Lens::ForEach => {
                if let Some(arr) = value.as_array_mut() {
                    arr.iter_mut()
                        .for_each(|item| self.set(item, new_value.clone()));
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_field() {
        let mut value = serde_json::json!({"a": 1});
        let lens = Lens::new("a");
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&serde_json::json!(1)));

        lens.set(&mut value, serde_json::json!(2));
        assert_eq!(value, serde_json::json!({"a": 2}));
    }

    #[test]
    fn test_index() {
        let mut value = serde_json::json!([1, 2, 3]);
        let lens = Lens::new(1);
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&serde_json::json!(2)));

        lens.set(&mut value, serde_json::json!(4));
        assert_eq!(value, serde_json::json!([1, 4, 3]));
    }

    #[test]
    fn test_compose() {
        let mut value = serde_json::json!({"a": [1, 2, 3]});
        let lens = Lens::default().select("a").select(1);
        let view = lens.get(&value).unwrap();
        assert_eq!(view, View::Borrow(&serde_json::json!(2)));

        lens.set(&mut value, serde_json::json!(4));
        assert_eq!(value, serde_json::json!({"a": [1, 4, 3]}));
    }

    #[test]
    fn test_for_each() {
        let mut value = serde_json::json!([{"a": 1}, {"a": 2}, {"a": 3}]);

        let lens = Lens::foreach().select("a");
        let view = lens.get(&value).unwrap();
        assert_eq!(
            view,
            View::BorrowVec(vec![
                View::Borrow(&serde_json::json!(1)),
                View::Borrow(&serde_json::json!(2)),
                View::Borrow(&serde_json::json!(3))
            ])
        );

        lens.set(&mut value, serde_json::json!(4));
        assert_eq!(value, serde_json::json!([{"a": 4}, {"a": 4}, {"a": 4}]));
    }
    #[test]
    fn test_deeply_nested() {
        let mut value = serde_json::json!({
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
        assert_eq!(view, View::Borrow(&serde_json::json!(3)));

        lens.set(&mut value, serde_json::json!(4));
        assert_eq!(
            value,
            serde_json::json!({
                "a": {
                    "b": {
                        "c": [1, 2, {"d": 4}]
                    }
                }
            })
        );
    }
}
