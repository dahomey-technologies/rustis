use serde::Deserialize;

/// Marker for a RESP Response
pub trait Response {}

impl<'de, T> Response for T where T: Deserialize<'de> {}
