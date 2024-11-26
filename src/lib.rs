use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
enum View<'a> {
    Borrow(&'a Value),
    BorrowVec(Vec<View<'a>>),
}

impl<'a> View<'a> {
    fn get(self, lens: &'a Lens) -> Option<Self> {
        match self {
            View::Borrow(value) => lens.get(value),
            View::BorrowVec(values) => {
                let mut new_values = Vec::new();
                for value in values {
                    if let Some(value) = value.get(lens) {
                        new_values.push(value);
                    }
                }
                if new_values.is_empty() {
                    None
                } else {
                    Some(View::BorrowVec(new_values))
                }
            }
        }
    }

    fn set(self, lens: &Lens, new_value: Value) -> Value {
        match self {
            View::Borrow(value) => {
                let mut value = value.clone();
                lens.set(&mut value, new_value);
                value
            }
            View::BorrowVec(values) => {
                let mut new_values = Vec::new();
                for value in values {
                    new_values.push(value.set(lens, new_value.clone()));
                }
                Value::Array(new_values)
            }
        }
    }
}

#[derive(Default, Debug)]
enum Lens {
    Field(String),
    Index(usize),
    Compose(Box<Lens>, Box<Lens>),
    ForEach,
    #[default]
    Empty,
}

impl Lens {
    fn get_mut<'a>(&'a self, value: &'a mut Value) -> Option<&'a mut Value> {
        match self {
            Lens::Field(field) => value.as_object_mut().and_then(|obj| obj.get_mut(field)),
            Lens::Index(index) => value.as_array_mut().and_then(|arr| arr.get_mut(*index)),
            Lens::Compose(first, second) => {
                if let Some(inner) = first.get_mut(value) {
                    second.get_mut(inner)
                } else {
                    None
                }
            }
            Lens::ForEach => None, // ForEach does not support get_mut
            Lens::Empty => Some(value),
        }
    }

    fn get<'a>(&'a self, value: &'a Value) -> Option<View<'a>> {
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

    fn set(&self, value: &mut Value, new_value: Value) {
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
                if let Some(inner) = first.get_mut(value) {
                    second.set(inner, new_value);
                }
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

    fn select<I: Select>(self, item: I) -> Self {
        item.pipe(self)
    }

    fn new<I: Select>(item: I) -> Self {
        item.pipe(Lens::Empty)
    }

    fn foreach() -> Self {
        Lens::ForEach
    }
}

trait Select {
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
        assert_eq!(value, serde_json::json!([{"a": 1}, {"a": 2}, {"a": 3}]));
    }
}
