use crate::{Pipeline, Cache};

pub trait ClientTrait : Send {
    fn create_pipeline(&mut self) -> Pipeline;    
    fn get_cache(&mut self) -> &mut Cache;
}