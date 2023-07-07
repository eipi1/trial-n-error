use core::fmt;
use std::collections::HashMap;
use std::marker::PhantomData;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Serialize, Serializer};
use serde::ser::Error;
use serde_json::Number;

macro_rules! tri {
    ($e:expr $(,)?) => {
        match $e {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => return core::result::Result::Err(err),
        }
    };
}

struct KeyClassifier;

enum KeyClass<'a> {
    Map(&'a str),
}

impl<'de> DeserializeSeed<'de> for KeyClassifier {
    type Value = KeyClass<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for KeyClassifier {
    type Value = KeyClass<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string key")
    }

    fn visit_borrowed_str<E>(self, s: &'de str) -> Result<Self::Value, E>
        where
            E: de::Error,
    {
        Ok(KeyClass::Map(s))
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(Number),
    Bytes(&'a [u8]),
    String(&'a str),
    Array(Vec<Value<'a>>),
    Object(HashMap<&'a str, Value<'a>>),
}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(n) => n.serialize(serializer),
            Value::Bytes(b) => serializer.serialize_str(std::str::from_utf8(b)
                .map_err(|_| Error::custom("Non-UTF encoding"))?),
            Value::String(s) => s.serialize(serializer),
            Value::Array(v) => v.serialize(serializer),
            Value::Object(m) => {
                use serde::ser::SerializeMap;
                let mut map = tri!(serializer.serialize_map(Some(m.len())));
                for (k, v) in m {
                    tri!(map.serialize_entry(k, v));
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value<'de> {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Value<'de>, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        struct ValueVisitor<'de> {
            marker: PhantomData<Value<'de>>,
            lifetime: PhantomData<&'de ()>,
        }

        impl<'de> Visitor<'de> for ValueVisitor<'de> {
            type Value = Value<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value<'de>, E> {
                Ok(Value::Bool(value))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value<'de>, E> {
                Ok(Value::Number(value.into()))
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value<'de>, E> {
                Ok(Value::Number(value.into()))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value<'de>, E> {
                Ok(Number::from_f64(value).map_or(Value::Null, Value::Number))
            }

            // #[cfg(any(feature = "std", feature = "alloc"))]
            #[inline]
            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Value<'de>, E>
                where
                    E: serde::de::Error,
            {
                println!("Visiting str -> ->");
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Value<'de>, E> where
                E: serde::de::Error,
            {
                println!("Visiting bytes -> ->");
                Ok(Value::Bytes(v))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value<'de>, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value<'de>, D::Error>
                where
                    D: serde::Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value<'de>, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value<'de>, V::Error>
                where
                    V: SeqAccess<'de>,
            {
                let mut vec = Vec::new();

                while let Some(elem) = tri!(visitor.next_element()) {
                    vec.push(elem);
                }

                Ok(Value::Array(vec))
            }

            // #[cfg(any(feature = "std", feature = "alloc"))]
            fn visit_map<V>(self, mut visitor: V) -> Result<Value<'de>, V::Error>
                where
                    V: MapAccess<'de>,
            {
                match visitor.next_key_seed(KeyClassifier)? {
                    Some(KeyClass::Map(first_key)) => {
                        let mut values = HashMap::new();

                        values.insert(first_key, tri!(visitor.next_value()));
                        while let Some((key, value)) = tri!(visitor.next_entry()) {
                            values.insert(key, value);
                        }

                        Ok(Value::Object(values))
                    }
                    None => Ok(Value::Object(HashMap::new())),
                }
            }
        }

        deserializer.deserialize_any(ValueVisitor { marker: PhantomData::<Value<'de>>, lifetime: PhantomData })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::Value;


    #[allow(dead_code)]
    #[test]
    fn serde_zero_copy_struct() {
        #[derive(Deserialize, Debug)]
        struct User<'a> {
            id: u32,
            name: &'a str,
            screen_name: &'a str,
            location: &'a str,
        }

        let json = r#"
        {
          "id": 123,
          "name": "John Doe",
          "screen_name": "Unidentified",
          "location": "Fringe"
        }
        "#;

        let result: User = serde_json::from_str(json).unwrap();
        let x = result.name.as_ptr();
        let x1 = json.as_ptr();
        let i = json.find("John").unwrap();
        let option = json.get(i..i + 8).unwrap().as_ptr();
        println!("{:?}", x);
        println!("{:?}", x1);
        println!("{:?}", option);
        let _ = dbg!(result);
    }

    #[test]
    fn serde_not_zero_copy_value() {
        let json_str = r#"{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe"}"#;
        let i = json_str.find("John").unwrap();
        let original_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        println!("original ptr: {:?}", original_ptr);
        let result: Value = serde_json::from_slice(json_str.as_bytes()).unwrap();
        println!("{:?}", result.get("name").unwrap().as_str().unwrap().as_ptr());
        let _ = dbg!(result);
    }

    #[test]
    fn serde_zero_copy_value_1() {
        let json_str = r#"{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Atlantis"}"#;
        let i = json_str.find("name").unwrap();
        let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        let i = json_str.find("John").unwrap();
        let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        println!("original ptr: {:?}", original_val_ptr);
        let result: super::Value = serde_json::from_slice(json_str.as_bytes()).unwrap();
        match &result {
            crate::Value::Object(obj) => {
                let (k, v) = obj.get_key_value("name").unwrap();
                assert_eq!(k.as_ptr(), original_key_ptr);
                match v {
                    crate::Value::String(s) => {
                        println!("{:?}", s);
                        assert_eq!(s.as_ptr(), original_val_ptr);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        dbg!(serde_json::to_string(&result).unwrap());
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
        let _ = dbg!(result);
    }

    #[test]
    fn serde_zero_copy_value_nested() {
        let json_str = r#"{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe","nested":{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe"}}"#;
        let i = json_str.find("name").unwrap();
        let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        let i = json_str.find("John").unwrap();
        let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        println!("original ptr: {:?}", original_val_ptr);
        let result: super::Value = serde_json_nostr::from_str(json_str).unwrap();
        match &result {
            crate::Value::Object(obj) => {
                let (k, v) = obj.get_key_value("name").unwrap();
                assert_eq!(k.as_ptr(), original_key_ptr);
                match v {
                    crate::Value::String(s) => {
                        println!("{:?}", s);
                        assert_eq!(s.as_ptr(), original_val_ptr);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
    }

    #[test]
    fn serde_zero_copy_value_with_bytes() {
        let json_str = r#"{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe","nested":{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe"}}"#;
        let i = json_str.find("name").unwrap();
        let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        let i = json_str.find("John").unwrap();
        let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        println!("original ptr: {:?}", original_val_ptr);
        let result: super::Value = serde_json_nostr::from_str(json_str).unwrap();
        match &result {
            crate::Value::Object(obj) => {
                let (k, v) = obj.get_key_value("name").unwrap();
                assert_eq!(k.as_ptr(), original_key_ptr);
                match v {
                    crate::Value::String(s) => {
                        println!("{:?}", s);
                        assert_eq!(s.as_ptr(), original_val_ptr);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
    }
}
