use std::fmt::{self,Display, Formatter};

#[derive(Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub flags: u8,
    pub attack: u16,
    pub defense: u16,
    pub regen: u16,
    pub health: i16,
    pub gold: u16,
    pub current_room: u16,
    pub description: String,
}

// initial points 40
impl Monster {
    pub fn new(name: String, description: String) -> Monster {
        Monster {
            name,
            flags: 0xF8,
            attack: 5,
            defense: 10,
            regen: 5,
            health: 20,
            gold: 0,
            current_room: 0,
            description,
        }
    }
}

impl Display for Monster {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "\n\tName: {}\n\tFlags: {:#02x}\n\tAttack: {}\n\tDefense: {}\n\tRegen: {}\n\tHealth: {}\n\tGold: {}\n\tRoom: {}", self.name, self.flags, self.attack, self.defense, self.regen, self.health, self.gold, self.current_room)
    }
}