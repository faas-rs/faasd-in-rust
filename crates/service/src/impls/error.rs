#[derive(Debug)]
pub enum ContainerdError {
    CreateContainerError(String),
    CreateSnapshotError(String),
    GetParentSnapshotError(String),
    GenerateSpecError(String),
    DeleteContainerError(String),
    GetContainerListError(String),
    KillTaskError(String),
    DeleteTaskError(String),
    WaitTaskError(String),
    CreateTaskError(String),
    StartTaskError(String),
    #[allow(dead_code)]
    OtherError,
}
