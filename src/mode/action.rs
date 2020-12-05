#[derive(Clone, Debug)]
pub struct LineNumber(pub usize);

#[derive(Clone, Debug)]
pub enum Action {
    AddInputLine(String),
    StartInsertingInput(LineNumber),
    StartAppendingInput(LineNumber),
    ChangeToCommandMode,
    Quit,
    SetLineNumber(usize),
}
