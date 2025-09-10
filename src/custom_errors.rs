use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScreenWarErrors {
    InvalidGlobalPDA,

    InvalidChallengePDA,

    InvalidUserPDA,

    InvalidPdaDataLen,

    ChallengeCreationPaused,

    ChallengeStartsTooSoon,

    ChallengeStartsTooFar,

    ChallengeExceedsTwoHours,

    JoinedLate,

    ContentionPhase,

    NotEnrolled,
}

impl From<ScreenWarErrors> for ProgramError {
    fn from(err: ScreenWarErrors) -> Self {
        ProgramError::Custom(err as u32)
    }
}
