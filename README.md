# typst_serde

Serde serializer for converting Rust types into Typst runtime values (`typst::foundations::Value`).

I am open to pull requests, however, the intent of the lib is to be very simple so I consider this feature complete.

---

## Features

- Serialize any `T: serde::Serialize` into `typst::foundations::Value`
- Convenience conversion to `typst::foundations::Dict` for map/struct-like inputs

---

## Quick start

```rust
use serde::Serialize;
use typst::foundations::{Str, Value};
use typst_serde::{to_dict, to_value, ToTypstValueExt};

#[derive(Serialize)]
struct Address {
    city: String,
    zip: u32,
}

#[derive(Serialize)]
struct Person {
    name: String,
    age: u32,
    active: bool,
    tags: Vec<String>,
    address: Address,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
        tags: vec!["admin".into(), "editor".into()],
        address: Address {
            city: "NYC".into(),
            zip: 10001,
        },
    };

    // Free-function APIs
    let value = to_value(&person)?;
    assert!(matches!(value, Value::Dict(_)));

    let dict = to_dict(&person)?;
    assert_eq!(
        dict.get(&Str::from("name")).unwrap(),
        &Value::Str(Str::from("Alice"))
    );

    // Trait method APIs (same result, method style)
    let value2 = person.to_typst_value()?;
    assert!(matches!(value2, Value::Dict(_)));

    let dict2 = person.to_typst_dict()?;
    assert_eq!(
        dict2.get(&Str::from("name")).unwrap(),
        &Value::Str(Str::from("Alice"))
    );

    Ok(())
}
```
