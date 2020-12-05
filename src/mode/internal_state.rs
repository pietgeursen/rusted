use ropey::Rope;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct InternalState {
    pub rope: Arc<Rope>,
    pub current_line: usize,
}

impl InternalState {
    pub fn new() -> InternalState {
        InternalState {
            rope: Arc::new(Rope::new()),
            current_line: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InputLinesToAdd{
    pub num_lines: usize,
    pub lines: String
}

