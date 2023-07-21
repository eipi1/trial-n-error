use core::fmt;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{de, Deserialize, Serialize, Serializer};
use serde::ser::Error;
use serde_json::Number;
use yoke_derive::Yokeable;

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

#[derive(Yokeable, Clone, Eq, PartialEq, Debug)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(Number),
    Bytes(&'a [u8]),
    String(&'a str),
    OwnedString(String),
    Array(Vec<Value<'a>>),
    // Object(HashMap<&'a str, Value<'a>>),
    Object(BTreeMap<&'a str, Value<'a>>),
}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(n) => n.serialize(serializer),
            Value::Bytes(b) => serializer.serialize_bytes(b),
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
            Value::OwnedString(s) => s.serialize(serializer),
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
                panic!();
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
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Value<'de>, E> where
                E: serde::de::Error,
            {
                Ok(Value::Bytes(v))
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Value<'de>, E> where
                E: serde::de::Error,
            {
                Ok(Value::OwnedString(String::from_utf8_lossy(v).into_owned()))
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
                        let mut values = BTreeMap::new();

                        values.insert(first_key, tri!(visitor.next_value()));
                        while let Some((key, value)) = tri!(visitor.next_entry()) {
                            values.insert(key, value);
                        }

                        Ok(Value::Object(values))
                    }
                    None => Ok(Value::Object(BTreeMap::new())),
                }
            }
        }

        deserializer.deserialize_any(ValueVisitor { marker: PhantomData::<Value<'de>>, lifetime: PhantomData })
    }
}


#[derive(Deserialize, Debug)]
// #[derive(Debug)]
struct User<'a> {
    id: u32,
    #[serde(with="serde_bytes")]
    name: &'a [u8],
    screen_name: &'a str,
    location: &'a str,
    // nested: &'a str,
}
/*
#[doc(hidden)]
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _: () = {
    #[allow(unused_extern_crates, clippy::useless_attribute)]
    extern crate serde as _serde;
    #[automatically_derived]
    impl<'de: 'a, 'a> _serde::Deserialize<'de> for User<'a> {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __field1,
                __field2,
                __field3,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        1u64 => _serde::__private::Ok(__Field::__field1),
                        2u64 => _serde::__private::Ok(__Field::__field2),
                        3u64 => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                {
                    match __value {
                        "id" => _serde::__private::Ok(__Field::__field0),
                        "name" => _serde::__private::Ok(__Field::__field1),
                        "screen_name" => _serde::__private::Ok(__Field::__field2),
                        "location" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                {
                    match __value {
                        b"id" => _serde::__private::Ok(__Field::__field0),
                        b"name" => _serde::__private::Ok(__Field::__field1),
                        b"screen_name" => _serde::__private::Ok(__Field::__field2),
                        b"location" => _serde::__private::Ok(__Field::__field3),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de: 'a, 'a> {
                marker: _serde::__private::PhantomData<User<'a>>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            impl<'de: 'a, 'a> _serde::de::Visitor<'de> for __Visitor<'de, 'a> {
                type Value = User<'a>;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(__formatter, "struct User")
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match match _serde::de::SeqAccess::next_element::<
                        u32,
                    >(&mut __seq) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct User with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field1 = match {
                        #[doc(hidden)]
                        struct __DeserializeWith<'de: 'a, 'a> {
                            value: &'a [u8],
                            phantom: _serde::__private::PhantomData<User<'a>>,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl<'de: 'a, 'a> _serde::Deserialize<'de>
                        for __DeserializeWith<'de, 'a> {
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private::Result<Self, __D::Error>
                                where
                                    __D: _serde::Deserializer<'de>,
                            {
                                _serde::__private::Ok(__DeserializeWith {
                                    value: match serde_bytes::deserialize(__deserializer) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                    phantom: _serde::__private::PhantomData,
                                    lifetime: _serde::__private::PhantomData,
                                })
                            }
                        }
                        _serde::__private::Option::map(
                            match _serde::de::SeqAccess::next_element::<
                                __DeserializeWith<'de, 'a>,
                            >(&mut __seq) {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            },
                            |__wrap| __wrap.value,
                        )
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    1usize,
                                    &"struct User with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field2 = match match _serde::de::SeqAccess::next_element::<
                        &'a str,
                    >(&mut __seq) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    2usize,
                                    &"struct User with 4 elements",
                                ),
                            );
                        }
                    };
                    let __field3 = match match _serde::de::SeqAccess::next_element::<
                        &'a str,
                    >(&mut __seq) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    3usize,
                                    &"struct User with 4 elements",
                                ),
                            );
                        }
                    };
                    _serde::__private::Ok(User {
                        id: __field0,
                        name: __field1,
                        screen_name: __field2,
                        location: __field3,
                    })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<u32> = _serde::__private::None;
                    let mut __field1: _serde::__private::Option<&'a [u8]> = _serde::__private::None;
                    let mut __field2: _serde::__private::Option<&'a str> = _serde::__private::None;
                    let mut __field3: _serde::__private::Option<&'a str> = _serde::__private::None;
                    while let _serde::__private::Some(__key)
                        = match _serde::de::MapAccess::next_key::<__Field>(&mut __map) {
                        _serde::__private::Ok(__val) => __val,
                        _serde::__private::Err(__err) => {
                            return _serde::__private::Err(__err);
                        }
                    } {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("id"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<u32>(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field1 => {
                                if _serde::__private::Option::is_some(&__field1) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("name"),
                                    );
                                }
                                __field1 = _serde::__private::Some({
                                    #[doc(hidden)]
                                    struct __DeserializeWith<'de: 'a, 'a> {
                                        value: &'a [u8],
                                        phantom: _serde::__private::PhantomData<User<'a>>,
                                        lifetime: _serde::__private::PhantomData<&'de ()>,
                                    }
                                    impl<'de: 'a, 'a> _serde::Deserialize<'de>
                                    for __DeserializeWith<'de, 'a> {
                                        fn deserialize<__D>(
                                            __deserializer: __D,
                                        ) -> _serde::__private::Result<Self, __D::Error>
                                            where
                                                __D: _serde::Deserializer<'de>,
                                        {
                                            _serde::__private::Ok(__DeserializeWith {
                                                value: match serde_bytes::deserialize(__deserializer) {
                                                    _serde::__private::Ok(__val) => __val,
                                                    _serde::__private::Err(__err) => {
                                                        return _serde::__private::Err(__err);
                                                    }
                                                },
                                                phantom: _serde::__private::PhantomData,
                                                lifetime: _serde::__private::PhantomData,
                                            })
                                        }
                                    }
                                    match _serde::de::MapAccess::next_value::<
                                        __DeserializeWith<'de, 'a>,
                                    >(&mut __map) {
                                        _serde::__private::Ok(__wrapper) => __wrapper.value,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    }
                                });
                            }
                            __Field::__field2 => {
                                if _serde::__private::Option::is_some(&__field2) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "screen_name",
                                        ),
                                    );
                                }
                                __field2 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        &'a str,
                                    >(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            __Field::__field3 => {
                                if _serde::__private::Option::is_some(&__field3) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field(
                                            "location",
                                        ),
                                    );
                                }
                                __field3 = _serde::__private::Some(
                                    match _serde::de::MapAccess::next_value::<
                                        &'a str,
                                    >(&mut __map) {
                                        _serde::__private::Ok(__val) => __val,
                                        _serde::__private::Err(__err) => {
                                            return _serde::__private::Err(__err);
                                        }
                                    },
                                );
                            }
                            _ => {
                                let _ = match _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map) {
                                    _serde::__private::Ok(__val) => __val,
                                    _serde::__private::Err(__err) => {
                                        return _serde::__private::Err(__err);
                                    }
                                };
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("id") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field1 = match __field1 {
                        _serde::__private::Some(__field1) => __field1,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                <__A::Error as _serde::de::Error>::missing_field("name"),
                            );
                        }
                    };
                    let __field2 = match __field2 {
                        _serde::__private::Some(__field2) => __field2,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("screen_name") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    let __field3 = match __field3 {
                        _serde::__private::Some(__field3) => __field3,
                        _serde::__private::None => {
                            match _serde::__private::de::missing_field("location") {
                                _serde::__private::Ok(__val) => __val,
                                _serde::__private::Err(__err) => {
                                    return _serde::__private::Err(__err);
                                }
                            }
                        }
                    };
                    _serde::__private::Ok(User {
                        id: __field0,
                        name: __field1,
                        screen_name: __field2,
                        location: __field3,
                    })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &[
                "id",
                "name",
                "screen_name",
                "location",
            ];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "User",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<User<'a>>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
 */
#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
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
        // match &result {
        //     crate::Value::Object(obj) => {
        //         let (k, v) = obj.get_key_value("name").unwrap();
        //         assert_eq!(k.as_ptr(), original_key_ptr);
        //         match v {
        //             crate::Value::String(s) => {
        //                 println!("{:?}", s);
        //                 assert_eq!(s.as_ptr(), original_val_ptr);
        //             }
        //             _ => {}
        //         }
        //     }
        //     _ => {}
        // }
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
                    // crate::Value::String(s) => {
                    //     println!("{:?}", s);
                    //     assert_eq!(s.as_ptr(), original_val_ptr);
                    // }
                    _ => {}
                }
            }
            _ => {}
        }
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
    }

    #[test]
    fn serde_zero_copy_large_value() {
        let mut file = std::fs::File::open("src/sample.json").unwrap();
        let mut contents = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut contents).unwrap();
        let string = String::from_utf8(contents).unwrap();
        let json_str = string.as_str();
        // let i = json_str.find("name").unwrap();
        // let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        // let i = json_str.find("John").unwrap();
        // let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        // println!("original ptr: {:?}", original_val_ptr);
        let result: super::Value = serde_json_nostr::from_str(json_str).unwrap();
        dbg!(&result);
        match &result {
            crate::Value::Object(obj) => {
                // let (k, v) = obj..get("product").get_key_value("allergens").unwrap();
                let (k, v) = obj.get_key_value("product").unwrap();

                // assert_eq!(k.as_ptr(), original_key_ptr);
                match v {
                    crate::Value::Object(obj) => {
                        let (k, v) = obj.get_key_value("allergens").unwrap();
                    }
                    _ => {
                        panic!()
                    }
                }
            }
            _ => {}
        }
        let value_str = serde_json_nostr::to_string(&result).unwrap();
        println!("{}", &value_str);
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&value_str).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
    }

    #[test]
    fn serde_zero_copy_value_with_bytes() {
        let json_str = r#"{"id":"123","name":"John Doe","screen_name":"Unidentified","location":"Fringe","nested":{"id":"123","name":"John Doe","screen_name":"Unidentified","location":"Fringe"}}"#;
        let i = json_str.find("name").unwrap();
        let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        let i = json_str.find("John").unwrap();
        let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();
        println!("original ptr: {:?}", original_val_ptr);
        let result: super::Value = serde_json::from_str(json_str).unwrap();
        match &result {
            crate::Value::Object(obj) => {
                let (k, v) = obj.get_key_value("name").unwrap();
                assert_eq!(k.as_ptr(), original_key_ptr);
                match v {
                    // crate::Value::String(s) => {
                    //     println!("{:?}", s);
                    //     assert_eq!(s.as_ptr(), original_val_ptr);
                    // }
                    _ => {}
                }
            }
            _ => {}
        }
        assert_json_diff::assert_json_eq!(serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&result).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap());
    }

    #[test]
    fn serde_bytes_test_json() {
        let json_str = r#"{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe","nested":{"id":123,"name":"John Doe","screen_name":"Unidentified","location":"Fringe"}}"#;
        let i = json_str.find("name").unwrap();
        let original_key_ptr = json_str.get(i..i + 4).unwrap().as_ptr();
        let i = json_str.find("John").unwrap();
        let original_val_ptr = json_str.get(i..i + 8).unwrap().as_ptr();


        let result: super::User = serde_json::from_str(json_str).unwrap();
        dbg!(result);
    }
}
