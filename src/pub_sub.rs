use crate::ConnectionMultiplexer;

#[derive(Clone)]
pub struct PubSub {
    pub(crate) multiplexer: ConnectionMultiplexer,
}

impl PubSub {
    pub(crate) fn new(multiplexer: ConnectionMultiplexer) -> Self {
        Self {
            multiplexer,
        }
    }
}