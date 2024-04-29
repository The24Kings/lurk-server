use std::sync::Arc;
use std::net::TcpStream;
use std::fmt::{self, Display, Formatter};

use crate::error_code::ErrorCode;

pub enum Message {
    /// # Type 1
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `message_len`: 2 bytes - 1-2
    /// 
    /// `recipient`: 32 bytes - 3-34
    /// 
    /// `sender`: 32 bytes - 35-66
    /// 
    /// `message`: variable length - 67+
    /// 
    /// Sent by the client to message other players. Can also be used by the server to send "presentable" information to the client 
    /// (information that can be displayed to the user with no further processing). Clients should expect to receive this type of message 
    /// at any time, and servers should expect to relay messages for clients at any time. If using this to send game information, 
    /// the server should mark the message as narration.
    Message {
        author: Arc<TcpStream>, 
        message_type: u8,
        message_len: u16,
        recipient: String,
        sender: String,
        message: String
    },
    /// # Type 2
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `room_num`: 2 bytes - 1-2
    /// 
    /// Sent by the client only, to change rooms. If the server changes the room a client is in, it should send an updated room, 
    /// character, and connection message(s) to explain the new location. If not, for example because the client is not ready to 
    /// start or specified an inappropriate choice, and error should be sent.
    /// 
    /// ## Note
    /// 
    /// Sequence for room entry: 
    /// 
    /// The server must accomplish a number of tasks when a player enters a room. The player should receive a ROOM, 
    /// an updated CHARACTER for the player who just entered the room, 0-n CHARACTER messages describing other players or monsters in the room, 
    /// and 0-n CONNECTION messages advertising connections from the current room. It should also send a CHARACTER message to every other player 
    /// in the room announcing the new entry into the room. The server should send the ROOM message to the player first, followed by the new 
    /// CHARACTER message showing the updated room, and then CONNECTION and CHARACTER in any order.
    ChangeRoom {
        author: Arc<TcpStream>, 
        message_type: u8,
        room_num: u16
    },
    /// # Type 3
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// Initiate a fight against monsters. This will start a fight in the current room against the monsters which are presently in the room. 
    /// Players with the join battle flag set, who are in the same room, will automatically join in the fight.
    Fight {
        author: Arc<TcpStream>, 
        message_type: u8,
    },
    /// # Type 4 (Optional)
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `target_name`: 32 bytes - 1-32
    /// 
    /// Initiate a fight against another player. The server will determine the results of the fight, and allocate damage and rewards appropriately. 
    /// The server may include players with join battle in the fight, on either side. Monsters may or may not be involved in the fight as well. 
    /// This message is sent by the client. If the server does not support PVP, it should send error 8 to the client.
    PVPFight {
        author: Arc<TcpStream>, 
        message_type: u8,
        target_name: String,
    },
    /// # Type 5
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `target_name`: 32 bytes - 1-32
    /// 
    /// Loot gold from a dead player or monster. The server may automatically gift gold from dead monsters to the players who have killed them, 
    /// or wait for a LOOT message. The server is responsible for communicating the results of the LOOT to the player, by sending an updated 
    /// CHARACTER message. This message is sent by the client.
    Loot {
        author: Arc<TcpStream>, 
        message_type: u8,
        target_name: String,
    },
    /// # Type 6
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// Start playing the game. A client will send a CHARACTER message to the server to explain character stats, which the server may either 
    /// accept or deny (by use of an ERROR message). If the stats are accepted, the server will not enter the player into the game world until 
    /// it has received START. This is sent by the client. Generally, the server will reply with a ROOM, a CHARACTER message showing the updated room, 
    /// and a CHARACTER message for each player in the initial room of the game.
    Start {
        author: Arc<TcpStream>, 
        message_type: u8
    },
    /// # Type 7
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `error`: 1 byte - 1
    /// 
    /// `message_len`: 2 bytes - 2-3
    /// 
    /// `message`: variable length - 4+
    /// 
    /// Notify the client of an error. This is used to indicate stat violations, inappropriate room connections, 
    /// attempts to loot nonexistent or living players, attempts to attack players or monsters in different rooms, etc.
    Error {
        author: Arc<TcpStream>,
        message_type: u8,
        error: ErrorCode,
        message_len: u16,
        message: Vec<u8>
    },
    /// # Type 8
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `accept_type`: 1 byte - 1
    /// 
    /// Sent by the server to acknowledge a non-error-causing action which has no other direct result. 
    /// This is not needed for actions which cause other results, such as changing rooms or beginning a fight. 
    /// It should be sent in response to clients sending messages, setting character stats, etc.
    Accept {
        author: Arc<TcpStream>,
        message_type: u8,
        accept_type: u8,
    },
    /// # Type 9
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `room_number`: 2 bytes - 1-2
    /// 
    /// `room_name`: 32 bytes - 3-34
    /// 
    /// `description_len`: 2 bytes - 35-36
    /// 
    /// `description`: variable length - 37+
    /// 
    /// Sent by the server to describe the room that the player is in. This should be an expected response to CHANGEROOM or START. 
    /// Can be re-sent at any time, for example if the player is teleported or falls through a floor. Outgoing connections 
    /// will be specified with a series of CONNECTION messages. Monsters and players in the room should be listed using a series of CHARACTER messages.
    Room { // Type 9
        message_type: u8,
        room_number: Vec<u8>, // Same as room_num in ChangeRoom
        room_name: Vec<u8>,
        description_len: u16,
        description: Vec<u8>,
    },
    /// # Type 10
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `name`: 32 bytes - 1-32
    /// 
    /// `flags`: 1 byte - 33
    /// 
    /// `attack`: 2 bytes - 34-35
    /// 
    /// `defense`: 2 bytes - 36-37
    /// 
    /// `regen`: 2 bytes - 38-39
    /// 
    /// `health`: 2 bytes - 40-41
    /// 
    /// `gold`: 2 bytes - 42-43
    /// 
    /// `current_room`: 2 bytes - 44-45
    /// 
    /// `description_len`: 2 bytes - 46-47
    /// 
    /// `description`: variable length - 48+
    ///
    /// Sent by both the client and the server. The server will send this message to show the client changes to 
    /// their player's status, such as in health or gold. The server will also use this message to show other players 
    /// or monsters in the room the player is in or elsewhere.
    /// 
    /// ## Note
    /// Flags:
    /// ```JSON
    ///     Alive: 1=alive, 0=dead
    ///     Join Battle: 1=join, 0=do not join
    ///     Monster: 1=monster, 0=player
    ///     Started: 1=started, 0=not started
    ///     Ready: 1=ready, 0=not ready
    /// ```
    /// When a client uses CHARACTER to describe a new player, the server may (should) ignore the client's initial 
    /// specification for health, gold, and room. The monster flag is used when describing monsters found in the 
    /// game rather than other human players.
    Character { // Type 10
        author: Arc<TcpStream>,
        message_type: u8,
        name: String,
        flags: u8,
        attack: u16,
        defense: u16,
        regen: u16,
        health: i16,
        gold: u16,
        current_room: u16,
        description_len: u16,
        description: Vec<u8>,
    },
    /// # Type 11
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `initial_points`: 2 bytes - 1-2
    /// 
    /// `stat_limit`: 2 bytes - 3-4
    /// 
    /// `description_len`: 2 bytes - 5-6
    /// 
    /// `description`: variable length - 7+
    /// 
    /// Used by the server to describe the game. The initial points is a combination of health, defense, and regen, 
    /// and cannot be exceeded by the client when defining a new character. The stat limit is a hard limit for the combination 
    /// for any player on the server regardless of experience. If unused, it should be set to 65535, the limit of the unsigned 16-bit integer. 
    /// This message will be sent upon connecting to the server, and not re-sent.
    Game {
        author: Arc<TcpStream>,
        message_type: u8,
        initial_points: u16,
        stat_limit: u16,
        description_len: u16,
        description: Vec<u8>,
    },
    /// # Type 12
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// Used by the client to leave the game. This is a graceful way to disconnect. The server never terminates, so it doesn't send LEAVE.
    Leave {
        author: Arc<TcpStream>,
        message_type: u8,
    },
    /// # Type 13
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `room_number`: 2 bytes - 1-2
    /// 
    /// `room_name`: 32 bytes - 3-34
    /// 
    /// `description_len`: 2 bytes - 35-36
    /// 
    /// `description`: variable length - 37+
    /// 
    /// Used by the server to describe rooms connected to the room the player is in. 
    /// The client should expect a series of these when changing rooms, but they may be sent at any time. 
    Connection {
        author: Arc<TcpStream>,
        message_type: u8,
        room_number: u16,
        room_name: u16,
        description_len: u16,
        description: Vec<u8>,
    },
    /// # Type 14
    /// 
    /// `author`: The client that sent the message
    /// 
    /// `message_type`: 1 byte - 0
    /// 
    /// `major_rev`: 1 byte - 1
    /// 
    /// `minor_rev`: 1 byte - 2
    /// 
    /// `extention_len`: 2 bytes - 3-4
    /// 
    /// `extensions`: variable length - 5+
    /// 
    /// Sent by the server upon initial connection along with GAME. If no VERSION is received, the server can be assumed to support only LURK 2.0 or 2.1.
    /// 
    /// ## Note
    /// At the end of the first extension, if there are more extensions, the length of the second extension will be found, then the second extension, and so on. 
    /// The length of the list of extensions must be the same as stated in the "size of the list of extensions" above.
    Version {
        author: Arc<TcpStream>,
        message_type: u8,
        major_rev: u8,
        minor_rev: u8,
        extention_len: u16, // Can be 0, just ignore
        extensions: Vec<u8>, // 0-1 length, 2-+ first extention;
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Message::Message { author, message_type, message_len, recipient, sender, message } => {
                write!(f, "Message {{ author: {:?}, message_type: {}, message_len: {:?}, recipient: {:?}, sender: {:?}, message: {:?} }}", author, message_type, message_len, recipient, sender, message)
            },
            Message::ChangeRoom { author, message_type, room_num } => {
                write!(f, "ChangeRoom {{ author: {:?}, message_type: {}, room_num: {:?} }}", author, message_type, room_num)
            },
            Message::Fight { author, message_type } => {
                write!(f, "Fight {{ author: {:?}, message_type: {} }}", author, message_type)
            },
            Message::PVPFight { author, message_type, target_name } => {
                write!(f, "PVPFight {{ author: {:?}, message_type: {}, target_name: {:?} }}", author, message_type, target_name)
            },
            Message::Loot { author, message_type, target_name } => {
                write!(f, "Loot {{ author: {:?}, message_type: {}, target_name: {:?} }}", author, message_type, target_name)
            },
            Message::Start { author, message_type } => {
                write!(f, "Start {{ author: {:?}, message_type: {} }}", author, message_type)
            },
            Message::Error { author, message_type, error, message_len, message } => {
                write!(f, "Error {{ author: {:?}, message_type: {}, error: {}, message_len: {}, message: {} }}", author, message_type, error, message_len, String::from_utf8_lossy(&message))
            },
            Message::Accept { author, message_type, accept_type } => {
                write!(f, "Accept {{ author: {:?}, message_type: {}, accept_type: {} }}", author, message_type, accept_type)
            },
            Message::Room { message_type, room_number, room_name, description_len, description } => {
                write!(f, "Room {{ message_type: {}, room_number: {:?}, room_name: {:?}, description_len: {:?}, description: {:?} }}", message_type, room_number, room_name, description_len, description)
            },
            Message::Character { author, message_type, name, flags, attack, defense, regen, health, gold, current_room, description_len, description } => {
                write!(f, "Character {{ author: {:?}, message_type: {}, name: {:?}, flags: {}, attack: {:?}, defense: {:?}, regen: {:?}, health: {:?}, gold: {:?}, current_room: {:?}, description_len: {:?}, description: {:?} }}", author, message_type, name, flags, attack, defense, regen, health, gold, current_room, description_len, description)
            },
            Message::Game { author, message_type, initial_points, stat_limit, description_len, description } => {
                write!(f, "Game {{ author: {:?}, message_type: {}, initial_points: {:?}, stat_limit: {:?}, description_len: {:?}, description: {:?} }}", author, message_type, initial_points, stat_limit, description_len, description)
            },
            Message::Leave { author, message_type } => {
                write!(f, "Leave {{ author: {:?}, message_type: {} }}", author, message_type)
            },
            Message::Connection { author, message_type, room_number, room_name, description_len, description } => {
                write!(f, "Connection {{ author: {:?}, message_type: {}, room_number: {:?}, room_name: {:?}, description_len: {:?}, description: {:?} }}", author, message_type, room_number, room_name, description_len, description)
            },
            Message::Version { author, message_type, major_rev, minor_rev, extention_len, extensions } => {
                write!(f, "Version {{ author: {:?}, message_type: {}, major_rev: {}, minor_rev: {}, extention_len: {:?}, extensions: {:?} }}", author, message_type, major_rev, minor_rev, extention_len, extensions)
            }
        }
    }
}