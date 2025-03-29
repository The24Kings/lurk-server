use std::fmt::{self,Display, Formatter};
use std::sync::Arc;
use std::net::TcpStream;

// Follow Character struct

#[derive(Debug, Clone)]
pub struct Character {
    pub conn: Arc<TcpStream>,
    pub active: bool,
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
impl Character {
    pub fn new(conn: Arc<TcpStream>, name: String, description: String) -> Character {
        Character {
            conn,
            active: true,
            name,
            flags: 0xff,
            attack: 5,
            defense: 10,
            regen: 5,
            health: 20,
            gold: 0,
            current_room: 0,
            description,
        }
    }

    pub fn update_room(&mut self, room: u16) {
        self.current_room = room;
    }

    pub fn update_connection(&mut self, conn: Arc<TcpStream>) {
        self.conn = conn;
    }
}

impl Display for Character {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "\n\tName: {}\n\tFlags: {:#02x}\n\tAttack: {}\n\tDefense: {}\n\tRegen: {}\n\tHealth: {}\n\tGold: {}\n\tRoom: {}", self.name, self.flags, self.attack, self.defense, self.regen, self.health, self.gold, self.current_room)
    }
}