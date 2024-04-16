use std::io::{prelude::*, BufReader};
use std::net::TcpStream;
use std::sync::Arc;
use std::result;
use std::sync::mpsc::Sender;

use crate::message::Message;
use crate::error_code::ErrorCode;
use crate::character::Character;

use crate::utilities::send_info;

type Result<T> = result::Result<T, ()>;

pub fn handle_client(stream: Arc<TcpStream>, messages: Sender<Message>, map_num: u8) -> Result<()> {
    let mut reader = BufReader::new(stream.as_ref());

    let mut message_type = [0u8];
    let mut buffer: Vec<u8> = Vec::new();

    // Server Constants
    let initial_points: u16 = 40;
    let stat_limit: u16 = 500;

    // Game Constants
    let mut started = false;
    let mut accepted_character = false;

    //Current character Constants
    let mut player: Character = Character::new(stream.clone(), String::new(), String::new());

    // Kill the thread if we can't get the peer address
    if stream.peer_addr().is_err() {
        eprintln!("[CLIENT]\tError: Could not get peer address of client; shutting down process.");

        return Err(());
    }

    // New character Connected
    println!("[CLIENT]\tNew connection: {:?}", stream.peer_addr());

    // Send game information to the client
    send_info(&stream, &messages, initial_points, stat_limit, map_num).map_err(|_err| {
        eprintln!("[CLIENT]\tError: Could not send game information to client");
    })?;

    // Listen for messages
    loop {
        // Read from the stream
        reader.read_exact(&mut message_type).map_err(|_err| {
            eprintln!("[CLIENT]\tError: Unable to recieve message_type, assuming character disconnected");

            let _ = messages.send(
                Message::Leave {
                    author: stream.clone(),
                    message_type: 12
                }
            );
        })?;

        let character_message: Message;

        // Read the rest of the message
        match message_type[0] {
            1 => {
                if !accepted_character || !started {
                    eprintln!("[CLIENT]\tError: Cannot send message to character when you haven't started the game yet!");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 33,
                        message: b"You haven't started the game yet!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Read the metadata
                let mut metadata = [0u8; 66];

                reader.read_exact(&mut metadata).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Unable to obtain metadata of message 1, assuming character disconnected");
                })?;

                // Remove the null bytes from the recipient and sender
                let mut r_bytes = metadata[2..33].to_vec();
                let mut s_bytes = metadata[34..65].to_vec();

                // For some reason the recipient metadata to incorrectly sent
                // so we need to find the first null byte and remove the rest of the bytes to get the recipient
                for i in 0..32 {
                    if metadata[i+2] == 0 {
                        r_bytes = metadata[2..i+2].to_vec();
                        break;
                    }
                }

                // Remove the null bytes from the sender
                s_bytes.retain(|&x| x != 0);

                let message_len = u16::from_le_bytes([metadata[0], metadata[1]]) as usize;
                let recipient = String::from_utf8_lossy(&r_bytes).to_string();
                let sender = String::from_utf8_lossy(&s_bytes).to_string();

                let mut message = vec![0u8; message_len];

                // Get the message
                reader.read_exact(&mut message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not read from stream, assuming character disconnected");
                })?;

                let message = String::from_utf8_lossy(&message).to_string();

                // Send the message to the server
                character_message = Message::Message {
                    author: stream.clone(),
                    message_type: 1,
                    message_len: message_len as u16,
                    recipient,
                    sender,
                    message
                };
            },
            2 => {
                if !accepted_character || !started {
                    eprintln!("[CLIENT]\tError: Cannot change room yet");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 33,
                        message: b"You haven't started the game yet!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Read the metadata
                let mut metadata = [0u8; 2];

                reader.read_exact(&mut metadata).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Unable to obtain metadata of message 2, assuming character disconnected");
                })?;

                let room_num = u16::from_le_bytes([metadata[0], metadata[1]]);

                // Send the change room message to the server
                character_message = Message::ChangeRoom {
                    author: stream.clone(),
                    message_type: 2,
                    room_num
                }
            },
            3 => {
                if !accepted_character || !started {
                    eprintln!("[CLIENT]\tError: Cannot fight if you haven't started the game yet!");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 33,
                        message: b"You haven't started the game yet!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Send the fight message to the server
                character_message = Message::Fight {
                    author: stream.clone(),
                    message_type: 3
                };
            },
            4 => {
                if !accepted_character || !started {
                    eprintln!("[CLIENT]\tError: Cannot fight players if you haven't started the game yet!");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 33,
                        message: b"You haven't started the game yet!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                //Disallow PVPFight
                let mut metadata = [0u8;32];

                reader.read_exact(&mut metadata).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Unable to obtain metadata of message 4, assuming character disconnected");
                })?;

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,    
                    error: ErrorCode::NoPlayerCombat,
                    message_len: 26,
                    message: b"Player PVP is not allowed!".to_vec()
                };
            },
            5 => {
                if !accepted_character || !started {
                    eprintln!("[CLIENT]\tError: Cannot loot if you haven't started the game yet!");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 33,
                        message: b"You haven't started the game yet!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Loot the room
                let mut metadata = [0u8; 32];

                reader.read_exact(&mut metadata).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Unable to obtain metadata of message 5, assuming character disconnected");
                })?;

                // Get target name
                let mut name_bytes = Vec::new();

                for i in 0..32 {
                    if metadata[i] == 0 {
                        name_bytes = metadata[0..i].to_vec();
                        break;
                    }
                }

                let target_name = String::from_utf8_lossy(&name_bytes).to_string();

                // Send the loot message to the server
                character_message = Message::Loot {
                    author: stream.clone(),
                    message_type: 5,
                    target_name
                };
            },
            6 => {
                if !accepted_character {
                    eprintln!("[CLIENT]\tError: Character not accepted");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::NotReady,
                        message_len: 34,
                        message: b"You must create a character first!".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Send the start message to the server
                character_message = Message::Start { 
                    author: stream.clone(), 
                    message_type: 6
                };

                started = true;
            },
            7 => {
                // An error message was sent by the client, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send an error message. Ignoring.");
                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 39,
                    message: b"I am the one who knocks, dont't try me!".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            8 => {
                // An Accept message was sent by the client, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send an accept message. Ignoring.");
                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);
               
                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 35,
                    message: b"Accept this disconnect you heathen.".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            9 => {
                // A room message was sent by the client, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send a room message. Ignoring.");
                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);
                
                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 52,
                    message: b"There isn't enough room here for the both of us pal.".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            10 => {
                // Read the metadata
                let mut metadata = [0u8; 47];

                reader.read_exact(&mut metadata).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Unable to obtain metadata of message 10, assuming character disconnected");
                })?;

                // Get character name
                let mut name_bytes = Vec::new();

                for i in 0..32 {
                    if metadata[i] == 0 {
                        name_bytes = metadata[0..i].to_vec();
                        break;
                    }
                }

                let temp_name = String::from_utf8_lossy(&name_bytes).to_string();
                let temp_flags = metadata[32];
                let temp_attack = u32::from_le_bytes([metadata[33], metadata[34], 0, 0]);
                let temp_defense = u32::from_le_bytes([metadata[35], metadata[36], 0, 0]);
                let temp_regen = u32::from_le_bytes([metadata[37], metadata[38], 0, 0]);
                let temp_health = i16::from_le_bytes([metadata[39], metadata[40]]);
                let desc_len = u16::from_le_bytes([metadata[45], metadata[46]]) as usize;
                let mut desc = vec![0u8; desc_len];

                // Get the description
                reader.read_exact(&mut desc).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not read from stream, assuming character disconnected");
                })?;

                // Calculate the total points
                let total_points = temp_attack + temp_defense + temp_regen;
 
                // Send Error if total points exceeds initial points
                if total_points > initial_points as u32 {
                    eprintln!("[CLIENT]\tError: Total points exceeds initial points");

                    character_message = Message::Error {
                        author: stream.clone(),
                        message_type: 7,
                        error: ErrorCode::StatError,
                        message_len: 35,
                        message: b"Total points exceeds initial points".to_vec()
                    };

                    messages.send(character_message).map_err(|_err| {
                        eprintln!("[CLIENT]\tError: Could not send error message to server");
                    })?;

                    continue;
                }

                // Set the character's stats
                if temp_name != "" { player.name = temp_name } else { player.name = "Default".to_string() };
                if temp_flags == 0x0 || temp_flags == 0xff {player.flags = 0xc8 } else { player.flags = temp_flags }; // 11001000 = 0xc8 (ready, not started) 11011000 = 0xd8 (ready, started)
                player.attack = temp_attack as u16;
                player.defense = temp_defense as u16;
                player.regen = temp_regen as u16;
                if temp_health == 0 { player.health = 20 } else { player.health = temp_health };
                player.gold = 0;
                player.current_room = 0;
                player.description = String::from_utf8_lossy(&desc).to_string();

                // Accept the character
                accepted_character = true;

                // Send the message to the server
                character_message = Message::Character {
                    author: stream.clone(),
                    message_type: 10,
                    name: player.name.clone(),
                    flags: player.flags,
                    attack: player.attack,
                    defense: player.defense,
                    regen: player.regen,
                    health: player.health,
                    gold: player.gold,
                    current_room: player.current_room,
                    description_len: desc_len as u16,
                    description: desc
                };
            },
            11 => {
                // Client tried to send a game message, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send a game message. Ignoring.");
                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 19,
                    message: b"Hey! That's my job!".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            12 => {
                // Send the leave message to the server and kill the thread
                character_message = Message::Leave {
                    author: stream.clone(),
                    message_type: 12
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send leave message to server");
                })?;
                
                break;
            },
            13 => {
                // Client tried to send a connection message, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send a connection message. Ignoring.");

                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 30,
                    message: b"Connect these hands, nice try!".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            14 => {
                // Client tried to send a version message, but we don't care about it
                eprintln!("[CLIENT]\tError: Client tried to send a version message. Ignoring.");
                let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 32,
                    message: b"Sorry no time traveling allowed!".to_vec()
                };

                messages.send(character_message).map_err(|_err| {
                    eprintln!("[CLIENT]\tError: Could not send error message to server");
                })?;

                break;
            },
            _ => {
                eprintln!("[CLIENT]\tError: Unknown message type: {}", message_type[0]);

                // Something very wrong has happened and we should disconnect this client.
                if message_type[0] > 14 {
                    eprintln!("[CLIENT]\tError: Message type out of acceptable range; disconnecting client.");
                    let _ = stream.as_ref().shutdown(std::net::Shutdown::Read);
                    break;
                }

                character_message = Message::Error {
                    author: stream.clone(),
                    message_type: 7,
                    error: ErrorCode::Other,
                    message_len: 20,
                    message: b"Unknown Message Type".to_vec()
                };
            }
        }   

        messages.send(character_message).map_err(|_err| {
            eprintln!("[CLIENT]\tError: Could not send character message to server");
        })?;

        buffer.clear();
    }
    
    Ok(())
}
