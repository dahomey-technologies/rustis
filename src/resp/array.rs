use crate::resp::Value;

#[derive(Debug, PartialEq)]
pub enum Array {
    Vec(Vec<Value>),
    Nil,
}
