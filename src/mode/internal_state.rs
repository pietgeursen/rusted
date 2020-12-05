use ropey::Rope;
use std::sync::Arc;
use std::fmt;
use futures::channel::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct InternalState {
    pub rope: Arc<Rope>,
    pub current_line: usize,
    pub output_writer: Option<UnboundedSender<Vec<u8>>>,
}

impl InternalState {
    pub fn new(output_writer: Option<UnboundedSender<Vec<u8>>>) -> InternalState {
        InternalState {
            rope: Arc::new(Rope::new()),
            current_line: 0,
            output_writer
        }
    }
}

impl fmt::Debug for InternalState{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
         .field("rope", &self.rope)
         .field("current_line", &self.current_line)
         .finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct InputLinesToAdd {
    pub num_lines: usize,
    pub lines: String,
}
