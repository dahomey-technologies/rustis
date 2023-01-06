pub trait Response {

}

pub struct ConcreteResponse<T> {
    pub data: T
}

impl<T> Response for ConcreteResponse<T> {

}