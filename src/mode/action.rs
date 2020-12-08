
#[derive(Clone, Debug)]
pub enum Action {
    AddInputLine(String),
    AddChar(String),
    Enter,
    StartInsertingInput,
    StartAppendingInput,
    ChangeToNormalMode,
    ChangeToCommandMode,
    Quit,
    SetLineNumber(usize),
}
