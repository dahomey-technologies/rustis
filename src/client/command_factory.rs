use crate::resp::CommandBuilder;

pub trait CommandFactory {
    fn cmd(&self, name: &'static str) -> CommandBuilder;
}