#[derive(Clone, Debug)]
pub struct LineNumber(pub usize);

#[derive(Clone, Debug)]
pub enum Action {
    AddInputLine(String),
    AddChar(String),
    Enter,
    StartInsertingInput(LineNumber),
    StartAppendingInput(LineNumber),
    ChangeToNormalMode,
    ChangeToCommandMode,
    Quit,
    SetLineNumber(usize),
}
