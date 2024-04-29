use std::io::prelude::*;
use std::sync::mpsc::Sender;
use std::result;
use std::sync::Arc;
use std::net::TcpStream;
use serde_json::Value;
use std::fs::File;

use crate::message::Message;
use crate::character::Character;
use crate::monster::Monster;

type Result<T> = result::Result<T, ()>;

/// Move the character to the given room
pub fn move_character(map: &mut Value, character: &Character, new_room_num: usize, old_room_num: usize) {
    // Remove the character from the old room
    let old_room = &mut map["rooms"][old_room_num]["characters"].as_array_mut();

    let old_room = match old_room {
        Some(room) => room,
        None => {
            eprintln!("[UTILS]\t\tError: Could not retrieve characters from old room");
            return;
        }
    };

    old_room.retain(|c| c.as_str().unwrap() != character.name);

    // Add the character to the new room
    let new_room = &mut map["rooms"][new_room_num]["characters"].as_array_mut();

    let room_names = match new_room {
        Some(room) => room,
        None => {
            eprintln!("[UTILS]\t\tError: Could not retrieve characters from new room");
            return;
        }
    };

    room_names.push(Value::String(character.name.clone()));
}

/// Send all players in a given room a character that just moved into/out of the room
pub fn alert_room(map: &Value, character: &Character, active_characters: &Vec<Character>, room_num: usize, old_room_num: usize) -> Result<()> {
    let room = &map["rooms"][room_num];

    // Get the players in the current room
    let characters = match room["characters"].as_array() {
        Some(names) => names,
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room characters");
            return Err(());
        }
    };

    let mut players: Vec<&str> = characters.iter().map(|c| c.as_str().unwrap()).collect();

    // Prevent sending the same characters the same message
    if old_room_num != room_num {
         // Get the players in the old room
        let old_room = &map["rooms"][old_room_num];

        let old_characters = match old_room["characters"].as_array() {
            Some(names) => names,
            None => {
                eprintln!("[UTILS]\t\tError: Could not get old room characters");
                return Err(());
            }
        };

        let old_players: Vec<&str> = old_characters.iter().map(|c| c.as_str().unwrap()).collect();

        // Add the old players to the list of players
        players.extend(old_players.iter());
    }   
    
    send_player_update_to_room(&players, character, active_characters).map_err(|_err| {
        eprintln!("[UTILS]\t\tError: Could not send all players in room a message");
    })?;

    Ok(())
}

/// Send the accept message to the author
pub fn send_accept(author: &Arc<TcpStream>) -> Result<()> {
    let message: Vec<u8> = [8,10].to_vec();

    author.as_ref().write_all(&message).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send accept message to character: {}", err);
    })?;

    Ok(())
}

/// Send the current character to the author
pub fn send_character(author: &Arc<TcpStream>, character: &Character) -> Result<()> {
    let mut message: Vec<u8> = Vec::new();

    let mut name = character.name.bytes().collect::<Vec<u8>>();
    let description = character.description.bytes().collect::<Vec<u8>>();
    let desc_len = description.len() as u16;

    // Resize the name to 32 bytes
    name.resize(32, 0);

    message.push(10);
    message.extend(name);
    message.push(character.flags);
    message.extend(character.attack.to_le_bytes());
    message.extend(character.defense.to_le_bytes());
    message.extend(character.regen.to_le_bytes());
    message.extend(character.health.to_le_bytes());
    message.extend(character.gold.to_le_bytes());
    message.extend(character.current_room.to_le_bytes());
    message.extend(desc_len.to_le_bytes());
    message.extend(description);

    // Send the character message to the author
    author.as_ref().write_all(message.as_slice()).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send character message to character: {}", err);
    })?;

    Ok(())
}

/// Send the current monster to the author
pub fn send_monster(author: &Arc<TcpStream>, monster: &Monster) -> Result<()> {
    let mut message: Vec<u8> = Vec::new();

    let mut name = monster.name.bytes().collect::<Vec<u8>>();
    let description = monster.description.bytes().collect::<Vec<u8>>();
    let desc_len = description.len() as u16;

    // Resize the name to 32 bytes
    name.resize(32, 0);

    message.push(10);
    message.extend(name);
    message.push(monster.flags);
    message.extend(monster.attack.to_le_bytes());
    message.extend(monster.defense.to_le_bytes());
    message.extend(monster.regen.to_le_bytes());
    message.extend(monster.health.to_le_bytes());
    message.extend(monster.gold.to_le_bytes());
    message.extend(monster.current_room.to_le_bytes());
    message.extend(desc_len.to_le_bytes());
    message.extend(description);

    // Send the monster message to the author
    author.as_ref().write_all(message.as_slice()).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send monster message to character: {}", err);
    })?;

    Ok(())
}

/// Send the current room to the author
pub fn send_room(author: &Arc<TcpStream>, map: &Value, active_characters: &Vec<Character>, active_monsters: &mut Vec<Monster>, room_num: usize, old_room_num: usize) -> Result<()> {
    let room = &map["rooms"][room_num];

    let mut message: Vec<u8> = Vec::new();

    let room_num = room_num as u16;
    let mut name: Vec<u8>;
    let description: Vec<u8>;
    let desc_len: u16;

    match room["name"].as_str() {
        Some(rm) => {
            name = rm.bytes().collect::<Vec<u8>>();

            // Resize the name to 32 bytes
            name.resize(32, 0);
        },
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room name");
            
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    }

    match room["description"].as_str() {
        Some(desc) => {
            description = desc.bytes().collect::<Vec<u8>>(); 
            desc_len = desc.len() as u16;
        },
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room description");
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    }

    println!("[UTILS]\t\tSending room: {}", room["name"].as_str().unwrap_or("ERROR"));

    message.push(9);
    message.extend(room_num.to_le_bytes());
    message.extend(name);
    message.extend(desc_len.to_le_bytes());
    message.extend(description); 

    // Send the ROOM message to the author
    author.as_ref().write_all(message.as_slice()).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send room message to character: {}", err);
    })?;  

    // Alert all players in the room
    println!("[UTILS]\t\tAlerting room of character movement from room {} to room {}.", old_room_num, room_num);
    let current_character = active_characters.iter().find(|c| Arc::ptr_eq(&c.conn, &author)).unwrap();

    alert_room(map, current_character, active_characters, room_num as usize, old_room_num as usize).map_err(|_err| {
        eprintln!("[UTILS]\t\tError: Could not alert room of character movement");
    })?;

    // Send the players and monsters in the room
    let mut players: Vec<&str> = Vec::new();
    let room_num = room_num as usize;

    // Get the players in the room
    let characters = match map["rooms"][room_num]["characters"].as_array() {
        Some(names) => names,
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room characters");
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    };

    players.extend(characters.iter().map(|c| c.as_str().unwrap()).collect::<Vec<&str>>());
    println!("[UTILS]\t\tPlayers in room: {:?}", players);

    // Send all players/monsters in the room to the author except for the author
    for player in players {
        let character = match active_characters.iter().find(|c| c.name == player) {
            Some(character) => character,
            None => {
                eprintln!("[UTILS]\t\tError: Could not get character from vector");
                author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                return Err(());
            }
        };

        send_character(author, &character)?;
    }
    
    println!("[UTILS]\t\tSent all players in room.");

    // Get the monster objects in the room
    let mut monsters = Vec::new();

    let enemies = match map["rooms"][room_num]["monsters"].as_array() {
        Some(enemies) => enemies,
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room monsters");
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    };

    // Log the monsters in the room
    monsters.extend(enemies.iter().map(|m| m.as_str().unwrap()).collect::<Vec<&str>>());

    println!("[UTILS]\t\tMonsters in room: {:?}", monsters);

    // Send all monsters in the room to the author
    for monster in monsters {
        let enemy = match active_monsters.iter().find(|m| m.name == monster) {
            Some(enemy) => enemy,
            None => {
                eprintln!("[UTILS]\t\tError: Could not get monster from map");
                author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                return Err(());
            }
        };

        send_monster(author, &enemy)?;
    }
    
    println!("[UTILS]\t\tSent all monsters in room.");

    Ok(())
}

/// Send the current connections of the given room to the author
pub fn send_connections(author: &Arc<TcpStream>, map: &Value, room_num: usize) -> Result<()> {
    let mut rooms = Vec::new();

    let cur_room = &map["rooms"][room_num];
    let connecting_rooms = match cur_room["exits"].as_array() {
        Some(exits) => exits.iter().map(|e| e.as_str().unwrap()).collect::<Vec<&str>>(),
        None => {
            eprintln!("[UTILS]\t\tError: Could not get room exits");
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    };

    let map_rooms = match map["rooms"].as_array() {
        Some(rooms) => rooms,
        None => {
            eprintln!("[UTILS]\t\tError: Could not get rooms");
            author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
            return Err(());
        }
    };

    rooms.extend(find_rooms(&map_rooms, &connecting_rooms));

    // Send the connecting rooms to the author
    for room in rooms {
        let mut message: Vec<u8> = Vec::new();

        let room_num = room["id"].as_u64().unwrap_or(99) as u16;
        let mut name: Vec<u8>;
        let description: Vec<u8>;
        let desc_len: u16;

        match room["name"].as_str() {
            Some(rm) => {
                name = rm.bytes().collect::<Vec<u8>>();
                name.resize(32, 0);
            },
            None => {
                eprintln!("[UTILS]\t\tError: Could not get room name from json object");
                author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                return Err(());
            }
        }

        match room["description"].as_str() {
            Some(desc) => {
                description = desc.bytes().collect::<Vec<u8>>(); 
                desc_len = desc.len() as u16;
            },
            None => {
                eprintln!("[UTILS]\t\tError: Could not get room description from json object");
                author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                return Err(());
            }
        }

        println!("[UTILS]\t\tSending connection: '{}'", room["name"].as_str().unwrap_or("ERROR"));

        message.push(13);
        message.extend(room_num.to_le_bytes());
        message.extend(name);
        message.extend(desc_len.to_le_bytes());
        message.extend(description); 

        // Send the ROOM message to the author
        author.as_ref().write_all(message.as_slice()).map_err(|err| {
            eprintln!("[UTILS]\t\tError: Could not send room message to character: {}", err);
        })?;
    }

    Ok(())
}

/// Send the client the version and description of the game
pub fn send_info(stream: &Arc<TcpStream>, messages: &Sender<Message>, initial_points: u16, stat_limit: u16, map_num: u8) -> Result<()> {
    // Send the version message and description
    let version_message = Message::Version {
        author: stream.clone(),
        message_type: 14,
        major_rev: 2,
        minor_rev: 3,
        extension_len: 0,
        extensions: Vec::new()
    };

    messages.send(version_message).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send version message to server: {}", err);

        std::process::exit(1);
    })?;

    // Send the description message
    let mut description_file = File::open(format!("/home/rjziegler/spring2024/cs435/lurk_server/description{}.txt", map_num)).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not read description file: {}", err);
    })?;

    let mut description = String::new();

    description_file.read_to_string(&mut description).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not read description file: {}", err);
    })?;
    
    let description_message = Message::Game {
        author: stream.clone(),
        message_type: 11,
        initial_points,
        stat_limit,
        description_len: description.len() as u16,
        description: description.as_bytes().to_vec()
    };

    messages.send(description_message).map_err(|err| {
        eprintln!("[UTILS]\t\tError: Could not send description message to server: {}", err);
    })?;

    Ok(())
}

/// Find the rooms that connect to the given room and return their json objects
pub fn find_rooms(map: &Vec<Value>, connecting_rooms: &Vec<&str>) -> Vec<Value> {
    let mut rooms = Vec::new();

    for room in map {
        let room_name = room["name"].as_str().unwrap_or("ERROR");

        if connecting_rooms.contains(&room_name) {
            rooms.push(room.clone());
        }
    }

    rooms
}

// Sends the character to all players in the room
pub fn send_player_update_to_room(players: &Vec<&str>, character: &Character, active_characters: &Vec<Character>) -> Result<()> {
    // Send all players in the room the character message
    for player in players {
        let receiptient = match active_characters.iter().find(|c| c.name == *player) {
            Some(character) => character,
            None => {
                eprintln!("[UTILS]\t\tError: Could not get character from map");
                return Err(());
            }
        };

        // Do not send the message to inactive characters
        if !receiptient.active {
            continue;
        }

        send_character(&receiptient.conn, character).map_err(|_err| {
            eprintln!("[UTILS]\t\tError: Could not send character to character");
        })?
    }

    Ok(())
}

/// Sends the monster to all players in the room
pub fn send_monster_update_to_room(players: &Vec<&str>, monster: &Monster, active_characters: &Vec<Character>) -> Result<()> {
    // Send all players in the room the monster message
    for player in players {
        let receiptient = match active_characters.iter().find(|c| c.name == *player) {
            Some(character) => character,
            None => {
                eprintln!("[UTILS]\t\tError: Could not get character from map");
                return Err(());
            }
        };

        // Do not send the message to inactive characters
        if !receiptient.active {
            continue;
        }

        send_monster(&receiptient.conn, monster).map_err(|_err| {
            eprintln!("[UTILS]\t\tError: Could not send monster to character");
        })?
    }

    Ok(())
}
