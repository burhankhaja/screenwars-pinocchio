use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScreenWarErrors {
    InvalidGlobalPDA,

    InvalidChallengePDA,

    ChallengeCreationPaused,

    ChallengeStartsTooSoon,

    ChallengeStartsTooFar,

    ChallengeExceedsTwoHours,
}

impl From<ScreenWarErrors> for ProgramError {
    fn from(err: ScreenWarErrors) -> Self {
        ProgramError::Custom(err as u32)
    }
}
