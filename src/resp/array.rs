use crate::resp::Value;

#[derive(Debug)]
pub enum Array {
    Vec(Vec<Value>),
    Nil,
}
