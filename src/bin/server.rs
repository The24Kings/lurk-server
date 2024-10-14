use std::sync::mpsc::sync_channel;
use dotenv::dotenv;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::{env, result, thread};
use serde_json::Value;

// Self-made modules
pub mod error_code;
pub mod message;
pub mod character;
pub mod monster;
pub mod client_thread;
pub mod server_thread;
pub mod utilities;

use crate::client_thread::handle_client;
use crate::server_thread::handle_server;
use crate::monster::Monster;

type Result<T> = result::Result<T, ()>;

// https://isoptera.lcsc.edu/~seth/cs435/lurk_2.3.html (Lurk Protocol)
// https://gamefaqs.gamespot.com/n64/197771-the-legend-of-zelda-ocarina-of-time/map/54?raw=1 (Game Map 2)

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("[MAIN]\t\tUsage: lurk-server <address> <port> [5050-5054] <map_num>");
        return Err(());
    }

    let address = format!("{}:{}", args[1], args[2]);
    let map_num = args[3].parse::<u8>().unwrap_or(1);

    let listener = TcpListener::bind(&address).map_err(|_err| {
        eprintln!("[MAIN]\t\tError: Could not bind to address {address}");
    })?;

    println!("Listening on {address}");

    let (message_sender, message_receiver) = sync_channel(0); // Create a synchronous channel

    let message_receiver = Arc::new(Mutex::new(message_receiver));

    match dotenv().ok() {
        Some(_) => {},
        None => {
            eprintln!("Error: Could not load .env file");
            return Err(());
        }
    }

    // Load environment variables
    let map_path = env::var("MAP_PATH").expect("MAP_PATH must be set");

    //Build the game map
    let map_file = File::open(format!("{}{}.json",map_path, map_num)).map_err(|err| {
        eprintln!("[MAIN]\t\tError: Could not read map file: {}", err);
    })?;

    let mut map: Value = serde_json::from_reader(map_file).map_err(|err| {
        eprintln!("[MAIN]\t\tError: Could not parse map file: {}", err);
    })?;

    // Load monsters
    let mut monsters: Vec<Monster> = map["monsters"].as_array().unwrap().iter().map(|monster| {
        Monster {
            name: monster["name"].as_str().unwrap_or("ERROR").to_string(),
            description: monster["description"].as_str().unwrap_or("SOMETHING WENT WRONG").to_string(),
            flags: 0xF8 as u8,
            attack: monster["attack"].as_u64().unwrap_or(0) as u16,
            defense: monster["defense"].as_u64().unwrap_or(0) as u16,
            regen: monster["regen"].as_u64().unwrap_or(0) as u16,
            health: monster["health"].as_u64().unwrap_or(0) as i16,
            gold: monster["gold"].as_u64().unwrap_or(0) as u16,
            current_room: monster["current_room"].as_u64().unwrap_or(0) as u16
        }
    }).collect();

    println!("[MAIN]\t\tLoaded {} monsters", monsters.len());
    
    // Spawn server thread
    println!("[MAIN]\t\tSpawning server thread");
    thread::spawn(move || handle_server(message_receiver, &mut map, &mut monsters)); //FIXME: Somewhere the channel is being closed before the server thread is done

    // Listen to incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream = Arc::new(stream);
                let message_sender = message_sender.clone();

                println!("[MAIN]\t\tNew connection; spawning client thread");
                thread::spawn(move || handle_client(stream, message_sender, map_num));
            }
            Err(e) => {
                eprintln!("[MAIN]\t\tError: {}\n", e);
            }
        }
    }

    Ok(())
}
