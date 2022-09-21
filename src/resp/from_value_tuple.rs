use crate::{
    resp::{Array, FromValue, Value},
    Error, Result,
};

impl<T1, T2> FromValue for (T1, T2)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (it.next(), it.next(), it.next()) {
                    (Some(v1), Some(v2), None) => Ok((v1.into()?, v2.into()?)),
                    (None, None, None) => Ok((Default::default(), Default::default())),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2>(tuple: (Result<T1>, Result<T2>)) -> Result<(T1, T2)> {
            let (v1, v2) = tuple;
            Ok((v1?, v2?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((first.into(), iter.next()?.into()))),
            }
        })
    }
}

impl<T1, T2, T3> FromValue for (T1, T2, T3)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (it.next(), it.next(), it.next(), it.next()) {
                    (Some(v1), Some(v2), Some(v3), None) => {
                        Ok((v1.into()?, v2.into()?, v3.into()?))
                    }
                    (None, None, None, None) => {
                        Ok((Default::default(), Default::default(), Default::default()))
                    }
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3>(
            tuple: (Result<T1>, Result<T2>, Result<T3>),
        ) -> Result<(T1, T2, T3)> {
            let (v1, v2, v3) = tuple;
            Ok((v1?, v2?, v3?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4> FromValue for (T1, T2, T3, T4)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (it.next(), it.next(), it.next(), it.next(), it.next()) {
                    (Some(v1), Some(v2), Some(v3), Some(v4), None) => {
                        Ok((v1.into()?, v2.into()?, v3.into()?, v4.into()?))
                    }
                    (None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4>(
            tuple: (Result<T1>, Result<T2>, Result<T3>, Result<T4>),
        ) -> Result<(T1, T2, T3, T4)> {
            let (v1, v2, v3, v4) = tuple;
            Ok((v1?, v2?, v3?, v4?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5> FromValue for (T1, T2, T3, T4, T5)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (Some(v1), Some(v2), Some(v3), Some(v4), Some(v5), None) => {
                        Ok((v1.into()?, v2.into()?, v3.into()?, v4.into()?, v5.into()?))
                    }
                    (None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        #[allow(clippy::complexity)]
        fn transpose<T1, T2, T3, T4, T5>(
            tuple: (Result<T1>, Result<T2>, Result<T3>, Result<T4>, Result<T5>),
        ) -> Result<(T1, T2, T3, T4, T5)> {
            let (v1, v2, v3, v4, v5) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5, T6> FromValue for (T1, T2, T3, T4, T5, T6)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
    T6: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (Some(v1), Some(v2), Some(v3), Some(v4), Some(v5), Some(v6), None) => Ok((
                        v1.into()?,
                        v2.into()?,
                        v3.into()?,
                        v4.into()?,
                        v5.into()?,
                        v6.into()?,
                    )),
                    (None, None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4, T5, T6>(
            tuple: (
                Result<T1>,
                Result<T2>,
                Result<T3>,
                Result<T4>,
                Result<T5>,
                Result<T6>,
            ),
        ) -> Result<(T1, T2, T3, T4, T5, T6)> {
            let (v1, v2, v3, v4, v5, v6) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?, v6?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5, T6, T7> FromValue for (T1, T2, T3, T4, T5, T6, T7)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
    T6: FromValue + Default,
    T7: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (
                        Some(v1),
                        Some(v2),
                        Some(v3),
                        Some(v4),
                        Some(v5),
                        Some(v6),
                        Some(v7),
                        None,
                    ) => Ok((
                        v1.into()?,
                        v2.into()?,
                        v3.into()?,
                        v4.into()?,
                        v5.into()?,
                        v6.into()?,
                        v7.into()?,
                    )),
                    (None, None, None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4, T5, T6, T7>(
            tuple: (
                Result<T1>,
                Result<T2>,
                Result<T3>,
                Result<T4>,
                Result<T5>,
                Result<T6>,
                Result<T7>,
            ),
        ) -> Result<(T1, T2, T3, T4, T5, T6, T7)> {
            let (v1, v2, v3, v4, v5, v6, v7) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?, v6?, v7?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8> FromValue for (T1, T2, T3, T4, T5, T6, T7, T8)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
    T6: FromValue + Default,
    T7: FromValue + Default,
    T8: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (
                        Some(v1),
                        Some(v2),
                        Some(v3),
                        Some(v4),
                        Some(v5),
                        Some(v6),
                        Some(v7),
                        Some(v8),
                        None,
                    ) => Ok((
                        v1.into()?,
                        v2.into()?,
                        v3.into()?,
                        v4.into()?,
                        v5.into()?,
                        v6.into()?,
                        v7.into()?,
                        v8.into()?,
                    )),
                    (None, None, None, None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4, T5, T6, T7, T8>(
            tuple: (
                Result<T1>,
                Result<T2>,
                Result<T3>,
                Result<T4>,
                Result<T5>,
                Result<T6>,
                Result<T7>,
                Result<T8>,
            ),
        ) -> Result<(T1, T2, T3, T4, T5, T6, T7, T8)> {
            let (v1, v2, v3, v4, v5, v6, v7, v8) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?, v6?, v7?, v8?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> FromValue for (T1, T2, T3, T4, T5, T6, T7, T8, T9)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
    T6: FromValue + Default,
    T7: FromValue + Default,
    T8: FromValue + Default,
    T9: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (
                        Some(v1),
                        Some(v2),
                        Some(v3),
                        Some(v4),
                        Some(v5),
                        Some(v6),
                        Some(v7),
                        Some(v8),
                        Some(v9),
                        None,
                    ) => Ok((
                        v1.into()?,
                        v2.into()?,
                        v3.into()?,
                        v4.into()?,
                        v5.into()?,
                        v6.into()?,
                        v7.into()?,
                        v8.into()?,
                        v9.into()?,
                    )),
                    (None, None, None, None, None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4, T5, T6, T7, T8, T9>(
            tuple: (
                Result<T1>,
                Result<T2>,
                Result<T3>,
                Result<T4>,
                Result<T5>,
                Result<T6>,
                Result<T7>,
                Result<T8>,
                Result<T9>,
            ),
        ) -> Result<(T1, T2, T3, T4, T5, T6, T7, T8, T9)> {
            let (v1, v2, v3, v4, v5, v6, v7, v8, v9) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?, v6?, v7?, v8?, v9?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> FromValue for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)
where
    T1: FromValue + Default,
    T2: FromValue + Default,
    T3: FromValue + Default,
    T4: FromValue + Default,
    T5: FromValue + Default,
    T6: FromValue + Default,
    T7: FromValue + Default,
    T8: FromValue + Default,
    T9: FromValue + Default,
    T10: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(values)) => {
                let mut it = values.into_iter();
                match (
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                    it.next(),
                ) {
                    (
                        Some(v1),
                        Some(v2),
                        Some(v3),
                        Some(v4),
                        Some(v5),
                        Some(v6),
                        Some(v7),
                        Some(v8),
                        Some(v9),
                        Some(v10),
                        None,
                    ) => Ok((
                        v1.into()?,
                        v2.into()?,
                        v3.into()?,
                        v4.into()?,
                        v5.into()?,
                        v6.into()?,
                        v7.into()?,
                        v8.into()?,
                        v9.into()?,
                        v10.into()?,
                    )),
                    (None, None, None, None, None, None, None, None, None, None, None) => Ok((
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        fn transpose<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>(
            tuple: (
                Result<T1>,
                Result<T2>,
                Result<T3>,
                Result<T4>,
                Result<T5>,
                Result<T6>,
                Result<T7>,
                Result<T8>,
                Result<T9>,
                Result<T10>,
            ),
        ) -> Result<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)> {
            let (v1, v2, v3, v4, v5, v6, v7, v8, v9, v10) = tuple;
            Ok((v1?, v2?, v3?, v4?, v5?, v6?, v7?, v8?, v9?, v10?))
        }
        Box::new(|iter| {
            let first = iter.next()?;
            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => Some(transpose((
                    first.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                    iter.next()?.into(),
                ))),
            }
        })
    }
}

