use std::fmt::{self,Display, Formatter};

pub enum ErrorCode {
    Other = 0,          // 0
    BadRoom = 1,        // 1
    PlayerExists = 2,   // 2
    BadMonster = 3,     // 3
    StatError = 4,      // 4
    NotReady = 5,       // 5
    NoTarget = 6,       // 6
    NoFight = 7,        // 7
    NoPlayerCombat = 8, // 8
}

impl Into<u8> for ErrorCode {
    fn into(self) -> u8 {
        self as u8
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ErrorCode::Other => write!(f, "Other"),
            ErrorCode::BadRoom => write!(f, "BadRoom"),
            ErrorCode::PlayerExists => write!(f, "PlayerExists"),
            ErrorCode::BadMonster => write!(f, "BadMonster"),
            ErrorCode::StatError => write!(f, "StatError"),
            ErrorCode::NotReady => write!(f, "NotReady"),
            ErrorCode::NoTarget => write!(f, "NoTarget"),
            ErrorCode::NoFight => write!(f, "NoFight"),
            ErrorCode::NoPlayerCombat => write!(f, "NoPlayerCombat"),
        }
    }
}