/// Stable, documented exit codes so CI consumers can branch on failure class
/// without parsing stderr text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    Generic = 1,
    ConfigValidation = 2,
    ApiNetwork = 3,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}
