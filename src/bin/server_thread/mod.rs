use std::sync::{Arc, Mutex};
use std::io::Write;
use serde_json::Value;
use std::sync::mpsc::Receiver;
use std::result;

use crate::message::Message;
use crate::character::Character;
use crate::error_code::ErrorCode;
use crate::monster::Monster;

use crate::utilities::{find_rooms, move_character, send_accept, send_character, send_connections, send_monster, send_room, send_player_update_to_room, send_monster_update_to_room};

type Result<T> = result::Result<T, ()>;

pub fn handle_server(message_receiver: Arc<Mutex<Receiver<Message>>>, map: &mut Value, active_monsters: &mut Vec<Monster>) -> Result<()> {
    let mut characters: Vec<Character> = Vec::new();
    
    loop {
        // Lock the message receiver
        let receiver = message_receiver.lock();

        // Receive a message
        let message = receiver.unwrap().recv().map_err(|err| {
            // Disconnect all characters
            for character in characters.iter_mut() {
                character.active = false;
                character.conn.as_ref().shutdown(std::net::Shutdown::Both).unwrap();
            }

            eprintln!("[SERVER]\tError: Could not receive message: {}\n", err);

            std::process::exit(1);
        })?;

        match message {
            Message::Message { author: _, message_type, message_len, recipient, sender, message } => {
                println!("[SERVER]\tReceived message from: {}", sender);
                println!("[SERVER]\tSending message to: {}", recipient);

                println!("[SERVER]\tMessage:\n\t{}", message);

                let mut server_message: Vec<u8> = Vec::new();

                // Resize the sender and recipient to 32 bytes
                let mut s_bytes = sender.bytes().collect::<Vec<u8>>();
                let mut r_bytes = recipient.bytes().collect::<Vec<u8>>();

                s_bytes.resize(32, 0);
                r_bytes.resize(32, 0);

                server_message.push(message_type);
                server_message.extend(message_len.to_le_bytes());
                server_message.extend(r_bytes);
                server_message.extend(s_bytes);
                server_message.extend(message.as_bytes());

                // Find the first recipient in the characters list
                let recipient = characters.iter().find(|c| c.name == recipient);

                match recipient {
                    Some(recipient) => {
                        // Send the message to the recipient
                        recipient.conn.as_ref().write_all(&server_message).map_err(|err| {
                            eprintln!("[SERVER]\tError: Could not send message to character: {}", err);
                        })?;
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Could not find recipient to message");
                    }
                }
            },
            Message::ChangeRoom { author, message_type: _, room_num } => {
                println!("[SERVER]\tReceived change room message from: {:?}", author.peer_addr());

                if characters.len() == 0 {
                    eprintln!("[SERVER]\tError: No characters in the list to change rooms");
                    continue;
                }

                // Send the updated character to the author
                let mut index = 0;

                for (i, character) in characters.iter().enumerate() {
                    if Arc::ptr_eq(&character.conn, &author) {
                        index = i;
                        break;
                    }
                }

                // Check if the character is dead
                if characters[index].health <= 0 || (characters[index].flags >> 7) & 1 == 0{
                    eprintln!("[SERVER]\tError: Character is dead and cannot change rooms");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::Other as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(39u16.to_le_bytes().to_vec());
                    message.extend(b"Player is dead and cannot change rooms!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                // Get exit names for the room and their ids
                let old_room_num: usize = characters[index].current_room as usize;

                let connection_names = match &map["rooms"][old_room_num]["exits"].as_array() {
                    Some(exits) => {
                        exits.iter().map(|e| e.as_str().unwrap()).collect::<Vec<&str>>()
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Could not get room exits");
                        author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                        return Err(());
                    }
                };

                // Get the connections for the room
                let all_rooms = &map["rooms"].as_array();
                let connections: Vec<Value>;
                let mut valid_connections: Vec<u16> = Vec::new();

                match all_rooms {
                    Some(rooms) => {
                        connections = find_rooms(rooms, &connection_names);
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Could not get rooms");
                        author.as_ref().shutdown(std::net::Shutdown::Read).unwrap_or_default();
                        return Err(());
                    }
                };

                // Get the connection ids
                for connection in connections {
                    let id = connection["id"].as_u64().unwrap_or(99) as u16;
                    let name = connection["name"].as_str().unwrap_or("Unknown");

                    println!("[SERVER]\tValid Connection: {} - '{}'", id, name);

                    valid_connections.push(id);
                }

                // Check if the room number is valid
                if !valid_connections.contains(&room_num) {
                    eprintln!("[SERVER]\tError: Invalid room number: {}", room_num);

                     // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();
                    let code: u8 = ErrorCode::BadRoom as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(31u16.to_le_bytes().to_vec());
                    message.extend(b"Not a valid room or connection!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                println!("[SERVER]\tMoving character to room: {}", room_num);

                // Move the character to the new room
                move_character(map, &characters[index], room_num as usize, old_room_num);

                // Update the characters room
                characters[index].update_room(room_num);

                // Send the new room to the author
                send_room(&author, &map, &characters, active_monsters, room_num as usize, old_room_num).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send room message to character");
                })?;

                // Send the connections to the author
                send_connections(&author, &map, room_num as usize).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send connections message to character");
                })?;
            },
            Message::Fight { author, message_type: _ } => {
                println!("[SERVER]\tReceived fight message from: {:?}", author.peer_addr());

                // Get information about the current room
                let initiator: &Character;
                let current_room: usize;
                let player_names: Vec<&str>;
                let monster_names: Vec<&str>;
                let mut players: Vec<&mut Character> = Vec::new();
                let mut monsters: Vec<&mut Monster> = Vec::new();

                let players_to_alert = characters.clone();

                // Find the character who sent the fight message
                match characters.iter().find(|c| Arc::ptr_eq(&c.conn, &author)) {
                    Some(character) => {
                        initiator = character;
                        current_room = character.current_room as usize;
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Could not find character who sent fight message");
                        continue;
                    }
                }

                // Check if initiator is dead
                if initiator.health <= 0 || (initiator.flags >> 7) & 1 == 0 {
                    eprintln!("[SERVER]\tError: Initiator is dead and cannot initiate a fight");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::Other as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(34u16.to_le_bytes().to_vec());
                    message.extend(b"Dead players cannot start battles!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                println!("[SERVER]\tFight Initiator: {}", initiator);

                // Get the players in the room
                match map["rooms"][current_room]["characters"].as_array() {
                    Some(players_array) => {
                        player_names = players_array.iter().map(|p| p.as_str().unwrap()).collect::<Vec<&str>>();
                    },
                    None => {
                        eprintln!("[SERVER]\tError: No players in the room to fight");
                        continue;
                    }
                }

                players.extend(
                    characters.iter_mut()
                        .filter(
                            |c| player_names.contains(&c.name.as_str())
                        )
                );

                // Get the monsters in the room
                match &map["rooms"][current_room]["monsters"].as_array() {
                    Some(monsters_array) => {
                       monster_names = monsters_array.iter().map(|m| m.as_str().unwrap()).collect::<Vec<&str>>();
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Unable to get monsters in the room");
                        continue;
                    }
                };

                monsters.extend(
                    active_monsters.iter_mut()
                        .filter(
                            |m| monster_names.contains(&m.name.as_str())
                        )
                );

                // Check if there are any monsters in the room
                if monsters.len() == 0 || (monsters.iter().all(|m| (m.flags >> 6) & 1 == 0)){
                    eprintln!("[SERVER]\tError: No monsters in the room to fight");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::Other as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(33u16.to_le_bytes().to_vec());
                    message.extend(b"No monsters in the room to fight!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                // Log the players and monsters joining the fight
                for player in players.iter() {
                    println!("[SERVER]\tPlayer joining fight: {}", player.name);
                }

                for monster in monsters.iter() {
                    println!("[SERVER]\tMonster joining fight: {}", monster.name);
                }

                /* 
                    Fight logic

                    Every player in the room will attack each monster
                    Every monster in the room will attack each player

                    Total damage - player/monster defense = actual damage taken 

                    Each player/monster regenerates health equal to 10% of their regen stat
                */

                // Pool player stats
                let mut total_player_damage: i64 = 0;

                players.iter().for_each(|player| {
                    if (player.flags >> 6) & 1 == 0 {
                        return;
                    }

                    total_player_damage += player.attack as i64;
                });

                // Pool monster stats
                let mut total_monster_damage: i64 = 0;

                monsters.iter().for_each(|monster| {
                    if (monster.flags >> 6) & 1 == 0 {
                        return;
                    }

                    total_monster_damage += monster.attack as i64;
                });

                // Calculate monster health
                for monster in monsters {
                    let damage = total_player_damage - monster.defense as i64;

                    if damage <= 0 {
                        println!("[SERVER]\tMonster: {} took no damage", monster.name);
                        continue;
                    }

                    if (monster.flags >> 6) & 1 == 0 {
                        println!("[SERVER]\tMonster: {} does not join fights", monster.name);
                        continue;
                    }

                    if monster.health <= 0 {
                        println!("[SERVER]\tMonster: {} is already dead", monster.name);
                        continue;
                    }

                    // Send Narration message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let mut r_bytes = monster.name.bytes().collect::<Vec<u8>>();
                    let mut s_bytes = "Server".bytes().collect::<Vec<u8>>();

                    r_bytes.resize(32, 0);
                    s_bytes.resize(32, 0);

                    let narration = format!("The players are attacking {}!", monster.name).as_bytes().to_vec();
                    let message_len = narration.len() as u16;

                    message.push(1);
                    message.extend(message_len.to_le_bytes());
                    message.extend(r_bytes);
                    message.extend(s_bytes);
                    message.extend(narration);

                    // Send the narration message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send narration message to character: {}", err);
                    })?;

                    println!("[SERVER]\tMonster: {} took {} damage", monster.name, damage as i16);

                    monster.health -= damage as i16;

                    // Regenerate health
                    let regen = monster.regen as f64 * 0.10;

                    println!("[SERVER]\tMonster: {} regenerated {} health", monster.name, regen as i16);

                    monster.health += regen as i16;

                    // Check if the monster is dead
                    if monster.health <= 0 {
                        println!("[SERVER]\tMonster: {} is dead", monster.name);

                        // Mark the monster as dead via flags
                        monster.flags = 0x38; // 00111000 = 0x38

                        // Remove the monster's attack from the total damage
                        total_monster_damage -= monster.attack as i64;
                        
                        println!("[SERVER]\tRemoved monster damage {}, remaining: {}", monster.attack as i64, total_monster_damage);
                    }

                    println!("[SERVER]\tSending monster update to room");

                    // Send the updated monster to the author
                    send_monster_update_to_room(&player_names, monster, &players_to_alert).map_err(|_err| {
                        eprintln!("[SERVER]\tError: Could not send monster update message to room");
                    })?;
                }

                // Calculate player health
                for player in players {
                    let damage = total_monster_damage - player.defense as i64;

                    if damage <= 0 {
                        println!("[SERVER]\tPlayer {} took no damage", player.name);
                        continue;
                    }

                    if (player.flags >> 6) & 1 == 0 {
                        println!("[SERVER]\tPlayer {} does not join fights", player.name);
                        continue;
                    }

                    if player.health <= 0 {
                        println!("[SERVER]\tPlayer {} is already dead", player.name);
                        continue;
                    }

                    // Send Narration message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let mut r_bytes = player.name.bytes().collect::<Vec<u8>>();
                    let mut s_bytes = "Server".bytes().collect::<Vec<u8>>();

                    r_bytes.resize(32, 0);
                    s_bytes.resize(32, 0);

                    let narration = format!("The monsters are attacking {}!", player.name).as_bytes().to_vec();
                    let message_len = narration.len() as u16;

                    message.push(1);
                    message.extend(message_len.to_le_bytes());
                    message.extend(r_bytes);
                    message.extend(s_bytes);
                    message.extend(narration);

                    // Send the narration message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send narration message to character: {}", err);
                    })?;

                    println!("[SERVER]\tPlayer {} took {} damage", player.name, damage);

                    player.health -= damage as i16;

                    // Regenerate health
                    let regen = player.regen as f64 * 0.10;

                    println!("[SERVER]\tPlayer {} regenerated {} health", player.name, regen);

                    player.health += regen as i16;

                    // Check if the player is dead
                    if player.health <= 0 {
                        println!("[SERVER]\tPlayer {} is dead", player.name);

                        // Mark the player as dead via flags
                        player.flags = 0x18; // 00011000 = 0x18

                        // Remove the player's attack from the total damage
                        total_player_damage -= player.attack as i64;
                    }

                    println!("[SERVER]\tSending player update to room");

                    // Send the updated player to the author
                    send_player_update_to_room(&player_names, player, &players_to_alert).map_err(|_err| {
                        eprintln!("[SERVER]\tError: Could not send player update message to room");
                    })?;
                }    
            },
            Message::Loot { author, message_type: _, target_name } => {
                println!("[SERVER]\tReceived loot message from: {:?}", author.peer_addr());
                println!("[SERVER]\tAttempting to loot target: {}", target_name);

                // Get character of the author
                let initiator = match characters.iter_mut().find(|c| Arc::ptr_eq(&c.conn, &author)) {
                    Some(initiator) => initiator,
                    None => {
                        eprintln!("[SERVER]\tError: Could not find initiator of loot message!");
                        continue;
                    }
                };
                
                // Get the current room
                let current_room = initiator.current_room as usize;

                // Check if the target is in the room
                let target = match active_monsters.iter_mut().find(|c| c.name == target_name) {
                    Some(target) => target,
                    None => {
                        eprintln!("[SERVER]\tError: Could not find target to loot");

                        // Send Error message to the author
                        let mut message: Vec<u8> = Vec::new();

                        let code: u8 = ErrorCode::BadMonster as u8;

                        message.push(7);
                        message.push(code);
                        message.extend(28u16.to_le_bytes().to_vec());
                        message.extend(b"Not a valid monster to loot!".to_vec());

                        // Send the error message to the author
                        author.as_ref().write_all(&message).map_err(|err| {
                            eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                        })?;

                        continue;
                    }
                };

                // Check if player is dead
                if initiator.health <= 0 {
                    eprintln!("[SERVER]\tError: Initiator is dead and cannot loot");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::Other as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(31u16.to_le_bytes().to_vec());
                    message.extend(b"Player is dead and cannot loot!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                // Check if the target is not dead
                if target.health > 0 {
                    eprintln!("[SERVER]\tError: Target is not dead and cannot be looted");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::BadMonster as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(31u16.to_le_bytes().to_vec());
                    message.extend(b"Monster is not dead and cannot be looted!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                // Check if the target has loot
                if target.gold == 0 {
                    eprintln!("[SERVER]\tError: Target has no loot");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::BadMonster as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(32u16.to_le_bytes().to_vec());
                    message.extend(b"Monster has already been looted!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }

                println!("[SERVER]\tPlayer: {} looted Monster: {} in room {} for {} gold!", initiator.name, target.name, current_room, target.gold);

                // Adjust stats for the initiator and target
                initiator.gold += target.gold;
                target.gold = 0;

                // Send the updated player to the author
                send_character(&author, &initiator).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send character message to character");
                })?;

                // Send the updated monster to the author
                send_monster(&author, &target).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send monster message to character");
                })?;
            }, 
            Message::Start { author, message_type: _ } => {
                println!("[SERVER]\tReceived start message from: {:?}", author.peer_addr());

                send_room(&author, &map, &characters, active_monsters, 0, 0).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send room message to character");
                })?;

                // Send the character to the author
                let character = match characters.iter_mut().find(|c| Arc::ptr_eq(&c.conn, &author)) {
                    Some(character) => character,
                    None => {
                        eprintln!("[SERVER]\tError: Could not find character to start");
                        continue;
                    }
                };
/* 
                // Check if the character is already started
                if (character.flags >> 3) & 1 == 1 { // 00001000
                    eprintln!("[SERVER]\tError: Character is already started");

                    // Send Error message to the author
                    let mut message: Vec<u8> = Vec::new();

                    let code: u8 = ErrorCode::Other as u8;

                    message.push(7);
                    message.push(code);
                    message.extend(28u16.to_le_bytes().to_vec());
                    message.extend(b"Character is already started!".to_vec());

                    // Send the error message to the author
                    author.as_ref().write_all(&message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                    })?;

                    continue;
                }
*/
                // Update the character flags to show that the character has started
                character.flags = 0xd8;

                println!("[SERVER]\tCharacter started: {}", character);

                send_connections(&author, &map, 0).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send connections message to character");
                })?;
            },
            Message::Error { author, message_type, error, message_len, message } => {
                println!("[SERVER]\tReceived error message from: {:?}", author.peer_addr());

                let mut server_message: Vec<u8> = Vec::new();
                let code: u8 = error.into();

                server_message.push(message_type);
                server_message.push(code);
                server_message.extend(message_len.to_le_bytes().to_vec());
                server_message.extend(message);

                // Send the error message to the author
                author.as_ref().write_all(&server_message).map_err(|err| {
                    eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                })?;
            },
            Message::Character { author, message_type: _, name, flags, attack, defense, regen, health, gold, current_room, description_len: _, description } => {
                println!("[SERVER]\tReceived character message from: {:?}", author.peer_addr());

                // Locate the character in the list
                let mut index = 0;
                let mut found = false;

                if let Some((i, _character)) = characters.iter().enumerate().find(|c| c.1.name == name) {
                    println!("[SERVER]\tCharacter found: {} at index {}", name, i);
                    index = i;
                    found = true;
                }

                // Check if the character is already in the list
                if found {
                    println!("[SERVER]\tCharacter already exists: {}", name);

                    // Check if the character is already active
                    if characters[index].active {
                        println!("[SERVER]\tCharacter is already active: {}", name);

                        let mut server_message: Vec<u8> = Vec::new();

                        server_message.push(7);
                        server_message.push(2);
                        server_message.extend(25u16.to_le_bytes());
                        server_message.extend(b"Character already exists!");

                        println!("[SERVER]\tSending playerExists error message to: {:?}", author.peer_addr());

                        // Send the error message to the author
                        author.as_ref().write_all(&server_message).map_err(|err| {
                            eprintln!("[SERVER]\tError: Could not send error message to character: {}", err);
                        })?;

                        continue;
                    }

                    println!("[SERVER]\tSending narration message to: {}", name);

                    // Send narration message to the author
                    let mut server_message: Vec<u8> = Vec::new();
                    let mut r_bytes = characters[index].name.bytes().collect::<Vec<u8>>();
                    let mut s_bytes = "Narrator".bytes().collect::<Vec<u8>>();

                    r_bytes.resize(32, 0);
                    s_bytes.resize(32, 0);

                    // Narration marker
                    s_bytes[30] = 0;
                    s_bytes[31] = 1;

                    // Get the starting room
                    let starting_room = map["rooms"][0]["name"].as_str().unwrap_or("Temple Entrance");

                    let message_len: u16 = if starting_room == "Temple Entrance" { 84 } else { 143 };

                    // Send the message to the author
                    server_message.push(1);
                    server_message.extend(message_len.to_le_bytes());
                    server_message.extend(r_bytes);
                    server_message.extend(s_bytes);

                    if starting_room == "Temple Entrance" {
                        server_message.extend(b"As you regain conciousness, you see a Wallmaster retreating into the darkness above.");
                    } else {
                        server_message.extend(b"You feel exhasted and groggy, you hear laughing and the sound of wood clacking together. A Skullkid must have dragged you back to the entrance.");
                    }

                    // Bring the character back to life
                    characters[index].active = true;
                    characters[index].flags = 0xc8; // 0xc8 = 11001000
                    characters[index].update_connection(author.clone());
                    characters[index].update_room(current_room);

                    println!("[SERVER]\tAccepted character: {}", characters[index]);

                    send_accept(&author).map_err(|_err| {
                        eprintln!("[SERVER]\tError: Could not send accept message to character");
                    })?;

                    // Send the character to the author
                    send_character(&author, &characters[index]).map_err(|_err| {
                        eprintln!("[SERVER]\tError: Could not send character message to character");
                    })?;
              
                    author.as_ref().write_all(&server_message).map_err(|err| {
                        eprintln!("[SERVER]\tError: Could not send message to character: {}", err);
                    })?;

                    continue;
                } 

                // Add the character to the list of characters
                let character = Character {
                    conn: author.clone(),
                    name: name.clone(),
                    active: true,
                    flags,
                    attack,
                    defense,
                    regen,
                    health,
                    gold,
                    current_room,
                    description: String::from_utf8_lossy(&description).to_string()
                };

                println!("[SERVER]\tCharacter added: {}", character);

                // Move the character to the starting room
                move_character(map, &character, 0, 0);

                // Add the character to the list
                characters.push(character);

                // Accept the character
                send_accept(&author).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send accept message to character");
                })?;

                // Send the character to the author
                send_character(&author, &characters[index]).map_err(|_err| {
                    eprintln!("[SERVER]\tError: Could not send character message to character");
                })?;
            },
            Message::Game { author, message_type, initial_points, stat_limit, description_len, description } => {
                match author.as_ref().peer_addr() {
                    Ok(addr) => {
                        println!("[SERVER]\tReceived game message from: {:?}", addr);
                    },
                    Err(err) => {
                        eprintln!("[SERVER]\tError: Could not get address of author: {}", err);

                        // Disconnect the client
                        match author.shutdown(std::net::Shutdown::Both) {
                            Ok(_) => {
                                println!("[SERVER]\tDisconnected Client");
                            },
                            Err(err) => {
                                eprintln!("[SERVER]\tError: Could not disconnect client: {}", err);
                            }
                        }

                        continue;
                    }
                }

                let mut message: Vec<u8> = Vec::new();

                message.push(message_type);
                message.extend(initial_points.to_le_bytes());
                message.extend(stat_limit.to_le_bytes());
                message.extend(description_len.to_le_bytes());
                message.extend(description);

                // Send the game message to the author
                author.as_ref().write_all(&message).map_err(|err| {
                    eprintln!("[SERVER]\tError: Could not send game message to character: {}", err);

                    // If Error is os 104; connection reset by peer, panic
                    if err.raw_os_error().unwrap_or(0) == 104 {
                        eprintln!("[SERVER]\tError: Connection reset by peer");
                        std::process::exit(1);
                    }
                })?;
            },
            Message::Leave { author, message_type: _ } => {
                match author.as_ref().peer_addr() {
                    Ok(addr) => {
                        println!("[SERVER]\tReceived leave message from: {:?}", addr);
                    },
                    Err(err) => {
                        eprintln!("[SERVER]\tError: Could not get address of author: {}", err);
                    }
                }

                // Find the character in the list and deactivate them
                let character = characters.iter_mut().find(|c| Arc::ptr_eq(&c.conn, &author));

                match character {
                    Some(character) => {
                        character.active = false;
                        character.flags = 0x00; // 0x00 = 00000000 Dead, Inactive, and Not in game
                    },
                    None => {
                        eprintln!("[SERVER]\tError: Could not find character to deactivate");
                    }
                };

                // Disconnect the client
                match author.shutdown(std::net::Shutdown::Both) {
                    Ok(_) => {
                        println!("[SERVER]\tDisconnected Client");
                    },
                    Err(err) => {
                        eprintln!("[SERVER]\tError: Could not disconnect client: {}", err);
                    }
                }
            },
            Message::Version { author, message_type, major_rev, minor_rev, extension_len: _, extensions: _ } => {
                match author.as_ref().peer_addr() {
                    Ok(addr) => {
                        println!("[SERVER]\tReceived version message from: {:?}", addr);
                    },
                    Err(err) => {
                        eprintln!("[SERVER]\tError: Could not get address of author: {}", err);

                        // Disconnect the client
                        match author.shutdown(std::net::Shutdown::Both) {
                            Ok(_) => {
                                println!("[SERVER]\tDisconnected Client");
                            },
                            Err(err) => {
                                eprintln!("[SERVER]\tError: Could not disconnect client: {}", err);
                            }
                        }

                        continue;
                    }
                }

                let mut message: Vec<u8> = Vec::new();

                message.push(message_type);
                message.extend(major_rev.to_le_bytes());
                message.extend(minor_rev.to_le_bytes());
                message.extend(0u16.to_le_bytes());

                // Send the version to the author
                author.as_ref().write_all(&message).map_err(|err| {
                    eprintln!("[SERVER]\tError: Could not send version message to character: {}", err);

                    // If Error is os 104; connection reset by peer, panic
                    if err.raw_os_error().unwrap_or(0) == 104 {
                        eprintln!("[SERVER]\tError: Connection reset by peer");
                        std::process::exit(1);
                    }
                })?;
            },
            _ => {
                eprintln!("[SERVER]\tError: Unsupported message type: {}", message);
            }
        }
    }
}  
