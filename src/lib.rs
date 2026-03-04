//! Serde serializer for converting Rust values into Typst runtime values.
//!
//! # Overview
//!
//! This crate provides a small `serde::Serializer` implementation that maps
//! Serde data-model values into `typst::foundations::Value`.
//!
//! ## Convenience APIs
//!
//! - [`to_value`] serializes any `Serialize` value into a Typst [`Value`].
//! - [`to_dict`] serializes and requires the output to be a Typst [`Dict`].
//!
//! ## Extension Trait
//!
//! The [`ToTypstValueExt`] trait adds method-style ergonomics:
//!
//! - `value.to_typst_value()`
//! - `value.to_typst_dict()`
//!
//! ## Numeric semantics
//!
//! Typst integers are represented by `i64`. Therefore:
//!
//! - `i8/i16/i32/i64` -> `Value::Int`
//! - `u8/u16/u32` -> `Value::Int`
//! - `u64` -> `Value::Int` **only** if `<= i64::MAX`, otherwise serialization errors
//! - `f32/f64` -> `Value::Float`
//!
//! This crate intentionally errors for `u64 > i64::MAX` to avoid silent precision
//! loss that would occur if large integers were converted to floating point.
//!
//! ## Map key semantics
//!
//! Typst dictionaries use string keys. This serializer accepts map keys that
//! serialize to:
//!
//! - `Value::Str`
//! - `Value::Int` (converted to decimal string)
//!
//! Other key types return an error.

use serde::ser::{self, Serialize};
use typst::foundations::{Array, Bytes, Dict, Str, Value};

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, SerError>;

/// Serialization error for Typst value conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerError(String);

impl std::fmt::Display for SerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SerError {}

impl ser::Error for SerError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

/// Serializes any Serde-compatible type into a Typst [`Value`].
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value> {
    value.serialize(TypstSerializer)
}

/// Serializes any Serde-compatible type into a Typst [`Dict`].
///
/// Returns an error if the serialized value is not a dictionary.
pub fn to_dict<T: Serialize + ?Sized>(value: &T) -> Result<Dict> {
    match to_value(value)? {
        Value::Dict(d) => Ok(d),
        other => Err(SerError(format!(
            "expected struct or map to serialize into Dict, got {:?}",
            other.ty()
        ))),
    }
}

/// Extension trait providing ergonomic method-style conversion.
///
/// # Example
///
/// ```rust
/// use serde::Serialize;
/// use typst_serde::ToTypstValueExt;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let p = Person {
///     name: "Alice".into(),
///     age: 30,
/// };
///
/// let value = p.to_typst_value().unwrap();
/// let dict = p.to_typst_dict().unwrap();
/// assert!(matches!(value, typst::foundations::Value::Dict(_)));
/// assert!(dict.get(&typst::foundations::Str::from("name")).is_ok());
/// ```
pub trait ToTypstValueExt: Serialize {
    /// Convert `self` into a Typst [`Value`].
    fn to_typst_value(&self) -> Result<Value> {
        to_value(self)
    }

    /// Convert `self` into a Typst [`Dict`].
    ///
    /// Returns an error if the serialized shape is not map/struct-like.
    fn to_typst_dict(&self) -> Result<Dict> {
        to_dict(self)
    }
}

impl<T: Serialize + ?Sized> ToTypstValueExt for T {}

/// The main Serde serializer that produces Typst `Value` instances.
#[derive(Debug, Copy, Clone)]
struct TypstSerializer;

impl ser::Serializer for TypstSerializer {
    type Ok = Value;
    type Error = SerError;
    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::Int(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::Int(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        if v <= i64::MAX as u64 {
            Ok(Value::Int(v as i64))
        } else {
            Err(SerError(format!(
                "u64 value {} exceeds Typst integer range (max {})",
                v,
                i64::MAX
            )))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::Float(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::Str(Str::from(v.to_string())))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::Str(Str::from(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::Bytes(Bytes::new(v.to_vec())))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::None)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::None)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        Ok(Value::None)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::Str(Str::from(variant)))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value> {
        let mut dict = Dict::new();
        dict.insert(Str::from(variant), value.serialize(TypstSerializer)?);
        Ok(Value::Dict(dict))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer {
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqSerializer {
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SeqSerializer {
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(TupleVariantSerializer {
            variant: Str::from(variant),
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer {
            dict: Dict::new(),
            current_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(StructSerializer { dict: Dict::new() })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(StructVariantSerializer {
            variant: Str::from(variant),
            dict: Dict::new(),
        })
    }
}

/// Serializer for sequences and arrays.
struct SeqSerializer {
    items: Vec<Value>,
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.items.push(value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(Array::from_iter(self.items)))
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.items.push(value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(Array::from_iter(self.items)))
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.items.push(value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Array(Array::from_iter(self.items)))
    }
}

/// Serializer for tuple variants like `Enum::Variant(a, b, c)`.
struct TupleVariantSerializer {
    variant: Str,
    items: Vec<Value>,
}

impl ser::SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.items.push(value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut dict = Dict::new();
        dict.insert(self.variant, Value::Array(Array::from_iter(self.items)));
        Ok(Value::Dict(dict))
    }
}

/// Serializer for maps with string-compatible keys.
struct MapSerializer {
    dict: Dict,
    current_key: Option<Str>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        let key_value = key.serialize(TypstSerializer)?;
        let key_str = match key_value {
            Value::Str(s) => s,
            Value::Int(i) => Str::from(i.to_string()),
            other => {
                return Err(SerError(format!(
                    "map keys must serialize to string or integer, got {:?}",
                    other.ty()
                )));
            }
        };
        self.current_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self
            .current_key
            .take()
            .ok_or_else(|| SerError("serialize_value called before serialize_key".into()))?;
        self.dict.insert(key, value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Dict(self.dict))
    }
}

/// Serializer for structs with named fields.
struct StructSerializer {
    dict: Dict,
}

impl ser::SerializeStruct for StructSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.dict
            .insert(Str::from(key), value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Dict(self.dict))
    }
}

/// Serializer for struct variants like `Enum::Variant { field: value }`.
struct StructVariantSerializer {
    variant: Str,
    dict: Dict,
}

impl ser::SerializeStructVariant for StructVariantSerializer {
    type Ok = Value;
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.dict
            .insert(Str::from(key), value.serialize(TypstSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut outer = Dict::new();
        outer.insert(self.variant, Value::Dict(self.dict));
        Ok(Value::Dict(outer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_bytes::ByteBuf;

    #[test]
    fn test_primitives() {
        assert_eq!(to_value(&true).unwrap(), Value::Bool(true));
        assert_eq!(to_value(&42i32).unwrap(), Value::Int(42));
        assert_eq!(to_value(&3.14f64).unwrap(), Value::Float(3.14));
        assert_eq!(to_value(&"hello").unwrap(), Value::Str(Str::from("hello")));
        assert_eq!(to_value(&'x').unwrap(), Value::Str(Str::from("x")));
    }

    #[test]
    fn test_option_and_unit() {
        assert_eq!(to_value(&None::<i32>).unwrap(), Value::None);
        assert_eq!(to_value(&Some(42)).unwrap(), Value::Int(42));
        assert_eq!(to_value(&()).unwrap(), Value::None);
    }

    #[test]
    fn test_vec_and_empty() {
        let vec = vec![1, 2, 3];
        let value = to_value(&vec).unwrap();
        match value {
            Value::Array(arr) => assert_eq!(arr.len(), 3),
            _ => panic!("expected array"),
        }

        let empty: Vec<i32> = vec![];
        let value = to_value(&empty).unwrap();
        match value {
            Value::Array(arr) => assert_eq!(arr.len(), 0),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_struct_and_to_dict() {
        #[derive(Serialize)]
        struct Person {
            name: String,
            age: u32,
        }

        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };

        let dict = to_dict(&person).unwrap();
        assert_eq!(
            dict.get(&Str::from("name")).unwrap(),
            &Value::Str(Str::from("Alice"))
        );
        assert_eq!(dict.get(&Str::from("age")).unwrap(), &Value::Int(30));
    }

    #[test]
    fn test_nested_struct() {
        #[derive(Serialize)]
        struct Address {
            city: String,
        }

        #[derive(Serialize)]
        struct Person {
            name: String,
            address: Address,
        }

        let person = Person {
            name: "Bob".to_string(),
            address: Address {
                city: "NYC".to_string(),
            },
        };

        let dict = to_dict(&person).unwrap();
        let address = dict.get(&Str::from("address")).unwrap();
        assert!(matches!(address, Value::Dict(_)));
    }

    #[test]
    fn test_enum_variants() {
        #[derive(Serialize)]
        enum Status {
            Active,
            Code(i32),
            Pair(i32, i32),
            Data { x: i32, y: i32 },
        }

        let unit = to_value(&Status::Active).unwrap();
        assert_eq!(unit, Value::Str(Str::from("Active")));

        let newtype = to_value(&Status::Code(7)).unwrap();
        match newtype {
            Value::Dict(d) => {
                assert_eq!(d.get(&Str::from("Code")).unwrap(), &Value::Int(7));
            }
            _ => panic!("expected dict"),
        }

        let tuple = to_value(&Status::Pair(1, 2)).unwrap();
        match tuple {
            Value::Dict(d) => {
                let v = d.get(&Str::from("Pair")).unwrap();
                assert!(matches!(v, Value::Array(_)));
            }
            _ => panic!("expected dict"),
        }

        let struct_variant = to_value(&Status::Data { x: 1, y: 2 }).unwrap();
        match struct_variant {
            Value::Dict(d) => {
                let inner = d.get(&Str::from("Data")).unwrap();
                assert!(matches!(inner, Value::Dict(_)));
            }
            _ => panic!("expected dict"),
        }
    }

    #[test]
    fn test_bytes() {
        let bytes = ByteBuf::from(vec![0u8, 1, 2, 255]);
        let value = to_value(&bytes).unwrap();
        assert!(matches!(value, Value::Bytes(_)));
    }

    #[test]
    fn test_u64_bounds() {
        assert_eq!(to_value(&(i64::MAX as u64)).unwrap(), Value::Int(i64::MAX));
        let err = to_value(&(i64::MAX as u64 + 1)).unwrap_err();
        assert!(err.to_string().contains("exceeds Typst integer range"));
    }

    #[test]
    fn test_map_key_restrictions() {
        use std::collections::BTreeMap;

        let mut ok: BTreeMap<i32, i32> = BTreeMap::new();
        ok.insert(1, 10);
        let v = to_value(&ok).unwrap();
        assert!(matches!(v, Value::Dict(_)));

        #[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
        struct K {
            a: i32,
        }

        let mut bad: BTreeMap<K, i32> = BTreeMap::new();
        bad.insert(K { a: 1 }, 10);
        let err = to_value(&bad).unwrap_err();
        assert!(
            err.to_string()
                .contains("map keys must serialize to string or integer")
        );
    }

    #[test]
    fn test_serde_rename() {
        #[derive(Serialize)]
        struct Renamed {
            #[serde(rename = "first_name")]
            val: String,
        }

        let r = Renamed { val: "Ada".into() };
        let dict = r.to_typst_dict().unwrap();
        assert_eq!(
            dict.get(&Str::from("first_name")).unwrap(),
            &Value::Str(Str::from("Ada"))
        );
    }

    #[test]
    fn test_extension_trait() {
        let x = 123i32;
        assert_eq!(x.to_typst_value().unwrap(), Value::Int(123));

        let map_like = serde_json::json!({ "k": 1 });
        let dict = map_like.to_typst_dict().unwrap();
        assert_eq!(dict.get(&Str::from("k")).unwrap(), &Value::Int(1));
    }

    #[test]
    fn test_to_dict_error_on_non_dict() {
        let err = to_dict(&vec![1, 2, 3]).unwrap_err();
        assert!(err.to_string().contains("expected struct or map"));
    }
}
