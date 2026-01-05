use std::fmt::{self, Write};

use crate::resp::CommandArgsIterator;
use bytes::Bytes;
use smallvec::SmallVec;

#[derive(Debug)]
pub struct NetworkCommand {
    buffer: Bytes,
    name_layout: (usize, usize),
    args_layout: SmallVec<[(usize, usize); 10]>,
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: usize,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
}

impl NetworkCommand {
    pub fn new(
        buffer: Bytes,
        name_layout: (usize, usize),
        args_layout: SmallVec<[(usize, usize); 10]>,
        #[cfg(debug_assertions)] kill_connection_on_write: usize,
        #[cfg(debug_assertions)] command_seq: usize,
    ) -> Self {
        Self {
            buffer,
            name_layout,
            args_layout,
            #[cfg(debug_assertions)]
            kill_connection_on_write,
            #[cfg(debug_assertions)]
            command_seq,
        }
    }

    pub fn get_bytes(&self) -> &Bytes {
        &self.buffer
    }

    pub fn get_name(&self) -> &[u8] {
        let (start, len) = self.name_layout;
        &self.buffer[start..start + len]
    }

    pub fn get_arg(&self, index: usize) -> Option<&[u8]> {
        let (start, len) = *self.args_layout.get(index)?;
        Some(&self.buffer[start..start + len])
    }

    pub fn args<'a>(&'a self) -> CommandArgsIterator<'a> {
        CommandArgsIterator {
            buffer: self.buffer.clone(),
            layout_iter: self.args_layout.iter(),
        }
    }
}

impl fmt::Display for NetworkCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        String::from_utf8_lossy(self.get_name()).fmt(f)?;
        for arg in self.args() {
            f.write_char(' ')?;
            String::from_utf8_lossy(&arg).fmt(f)?;
        }

        Ok(())
    }
}
