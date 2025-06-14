use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Reference(String),
    Null,
}

#[derive(Debug, PartialEq)]
pub enum ValueRef<'a> {
    String(&'a str),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Array(Vec<ValueRef<'a>>),
    Object(HashMap<&'a str, ValueRef<'a>>),
    Reference(&'a str),
    Null,
}

impl<'a> ValueRef<'a> {
    pub fn to_value(self) -> Value {
        match self {
            ValueRef::String(x) => Value::String(x.to_string()),
            ValueRef::Integer(x) => Value::Integer(x),
            ValueRef::Number(x) => Value::Number(x),
            ValueRef::Boolean(x) => Value::Boolean(x),
            ValueRef::Array(value_refs) => Value::Array(
                value_refs
                    .into_iter()
                    .map(|x| ValueRef::to_value(x))
                    .collect(),
            ),
            ValueRef::Object(hash_map) => Value::Object(
                hash_map
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), ValueRef::to_value(v)))
                    .collect(),
            ),
            ValueRef::Reference(x) => Value::Reference(x.to_string()),
            ValueRef::Null => Value::Null,
        }
    }
}
