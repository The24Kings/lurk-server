use std::io;
use std::env;
use std::sync::Arc;
use std::thread;
use std::fs::File;
use std::io::BufReader;
use std::time::{ Duration, Instant };
use std::net::TcpStream;
use crossterm::cursor::{ MoveTo, Hide, Show };
use crossterm::QueueableCommand;
use std::sync::mpsc::{ channel, Sender };
use std::io::{ stdout, Error, Read, Result, Write };
use crossterm::event::{ poll, read, Event, KeyCode, KeyModifiers };
use crossterm::terminal::{ self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen };

pub mod message;
pub mod error_code;

use crate::message::Message;

struct Window {
    scroll_ptr: usize,
    x: u16,
    y: u16,
    w: u16,
    h: u16
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage:\n\t\tlurkKnight <address> <port>");
        return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid number of arguments"));
    }

    let address = format!("{}:{}", args[1], args[2]);
    let stream: Arc<TcpStream>;

    // Crate mscp channel
    let (message_sender, message_receiver) = channel();

    // Load Logo
    let mut logo = File::open("/home/rjziegler/spring2024/cs435/lurk_server/logo.txt").map_err(|err| {
        eprintln!("Error: Could not read logo file: {}", err);
        io::Error::new(io::ErrorKind::InvalidData, "Could not read logo file")
    })?;

    let mut logo_txt = String::new();
    logo.read_to_string(&mut logo_txt).expect("Could not read logo file");

    // Output buffer
    let mut output: Vec<String> = Vec::new();
    
    // Set up the terminal
    stdout().queue(EnterAlternateScreen).unwrap();
    let _ = terminal::enable_raw_mode();

    let (mut w, mut h) = terminal::size().unwrap();

    if w < 130 || h < 35 {
        eprintln!("Error: Terminal must be at least 130x35");
        clean_up(stdout().by_ref());

        return Err(Error::new(io::ErrorKind::InvalidData, "Terminal must be at least 130x35"));
    }
    
    let mut seperator = "\x1b[32m=\x1b[0m".repeat(w as usize);
    let mut prompt = String::new();
    let mut user_input = "\x1b[36m> \x1b[0m ".to_string();
    let temp_input = "\x1b[36m> \x1b[0m ".to_string();
    let blank_input = "   ".to_string();

    let mut start_time = Instant::now();
    let mut check_time: Duration;
    let blink_delay = Duration::from_millis(500);

    let mut stop = false;
    let mut blink = false;

    let mut main_window = Window {
        scroll_ptr: 0,
        x: 0,
        y: 0,
        w,
        h: h - 2
    };

    /* { Misc. User Messages } */
    push_to_output(&mut output, format!("\x1b[32mConnecting to host {}...\x1b[0m", address), &mut main_window);

    // Connect to the server
    match TcpStream::connect(&address) {
        Ok(s) => {
            stream = Arc::new(s);
            push_to_output(&mut output, String::from("\x1b[92mConnected!\x1b[0m\n\n"), &mut main_window);
        },
        Err(e) => {
            eprintln!("Error: Could not connect to server: {}", e);
            clean_up(stdout().by_ref());

            return Err(e);
        }
    }

    // Clear the screen
    stdout().queue(Hide).unwrap();
    stdout().queue(Clear(ClearType::All)).unwrap();

    // Push the logo to the output buffer
    push_to_output(&mut output, logo_txt, &mut main_window);

    // TODO: Remove this later -> Warning
    push_to_output(&mut output, String::from("\x1b[31mWARNING\x1b[0m -> \x1b[4mThis client is still heavily under development\x1b[0m <- \x1b[31mWARNING\x1b[0m\n\n"), &mut main_window);

    thread::spawn(move || listen_to_server(&stream, message_sender));

    /* { Main Loop } */
    while !stop {
        while poll(Duration::ZERO).unwrap() {
            match read().unwrap() {
                Event::Resize(nw, nh) => {
                    w = nw;
                    h = nh;

                    main_window.h = nh - 2;

                    seperator = "\x1b[32m=\x1b[0m".repeat(w as usize);

                    // Clear the screen
                    stdout().queue(Clear(ClearType::All)).unwrap();
                },
                Event::Key(event) => {
                    match event.code {
                        KeyCode::Char(c) => {
                            // Check if ctrl+c was pressed
                            if c == 'c' && event.modifiers.contains(KeyModifiers::CONTROL) {
                                stop = true;
                            } else {
                                prompt.push(c);
                            }
                        },
                        KeyCode::Enter => {
                            //TODO: Process the type and send it to the server
                            push_to_output(&mut output, prompt.clone(), &mut main_window);
                            
                            main_window.scroll_ptr = output.len();
                            prompt.clear();
                        },
                        KeyCode::Backspace => {
                            prompt.pop();
                        },
                        KeyCode::Up => {
                            if main_window.scroll_ptr > (0 + main_window.h as usize) {
                                main_window.scroll_ptr -= 1;
                            }
                        },
                        KeyCode::Down => {
                            if main_window.scroll_ptr < output.len() {
                                main_window.scroll_ptr += 1;
                            }
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        /* { Read from server } */
        match message_receiver.try_recv() {
            Ok(message) => {
                match message {
                    Message::Message { author: _, message_type, message_len: _, recipient, sender, message } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (MESSAGE)\n\
                                Author: {}\n\
                                Recipient: {}\n\
                                Message: {}\n\n", 
                                message_type, sender, recipient, message
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Error { author: _, message_type, error, message_len: _, message } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (ERROR)\n\
                                Error Code: {}\n\
                                Error Message: {}\n\n", 
                                message_type, error, String::from_utf8(message).unwrap()
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Accept { author: _, message_type, accept_type} => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (ACCEPT)\n\
                                Accept Type: {}\n\n", 
                                message_type, accept_type
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Room { message_type, room_number, room_name, description_len: _, description } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (ROOM)\n\
                                Room Number: {}\n\
                                Room Name: {}\n\
                                Description: {}\n\n", 
                                message_type, u16::from_le_bytes([room_number[0],room_number[1]]), String::from_utf8(room_name).unwrap(), String::from_utf8(description).unwrap()
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Character { author: _, message_type, name, flags, attack, defense, regen, health, gold, current_room, description_len: _, description } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (CHARACTER)\n\
                                Name: {}\n\
                                Flags: {:#02x}\n\
                                Attack: {}\n\
                                Defense: {}\n\
                                Regen: {}\n\
                                Health: {}\n\
                                Gold: {}\n\
                                Current Room: {}\n\
                                Description: {}\n\n", 
                                message_type, name, flags, attack, defense, regen, health, gold, current_room, String::from_utf8(description).unwrap()
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Game { author: _, message_type, initial_points, stat_limit, description_len: _, description } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (GAME)\n\
                                Initial Points: {}\n\
                                Stat Limit: {}\n\
                                Description: {}\n\n", 
                                message_type, initial_points, stat_limit, String::from_utf8(description).unwrap()
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Connection { author: _, message_type, room_number, room_name, description_len: _, description } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (CONNECTION)\n\
                                Room Number: {}\n\
                                Room Name: {}\n\
                                Description: {}\n\n", 
                                message_type, room_number, String::from_utf8(room_name).unwrap(), String::from_utf8(description).unwrap()
                            ), 
                            &mut main_window
                        );
                    },
                    Message::Version { author: _, message_type, major_rev, minor_rev, extension_len, extensions: _ } => {
                        push_to_output(
                            &mut output, 
                            format!(
                                "\x1b[32mType\x1b[0m: {} (VERSION)\n\
                                Major Revision: {}\n\
                                Minor Revision: {}\n\
                                Extensions: {}\n\n", 
                                message_type, major_rev, minor_rev, extension_len
                            ), 
                            &mut main_window
                        );
                    }
                    _ => {}
                }
            },
            Err(err) => {
                if err == std::sync::mpsc::TryRecvError::Disconnected {
                    eprintln!("Error: Listening thread crashed");
                    clean_up(stdout().by_ref());

                    return Err(Error::new(io::ErrorKind::Other, "Listening thread crashed"));
                }
            }
        };

        /* { Render the screen } */
        stdout().queue(Clear(ClearType::UntilNewLine)).unwrap();

        chat_window(&mut stdout(), &output[..main_window.scroll_ptr], &main_window);

        // Draw the seperator
        stdout().queue(MoveTo(0, h-2)).unwrap();
        stdout().write(seperator.as_bytes()).unwrap();

        // Move to input line
        stdout().queue(MoveTo(0, h-1)).unwrap();
        stdout().write(user_input.as_bytes()).unwrap();

        // Write the prompt
        let bytes = prompt.as_bytes();
        stdout().write(bytes.get(0..w as usize).unwrap_or(bytes)).unwrap();

        // Flush the output
        stdout().flush().unwrap();

        // Check how much time has passed
        check_time = start_time.elapsed();

        // Blink the cursor
        if check_time >= blink_delay {
            blink = !blink;
            start_time = Instant::now();
        } 

        if blink {
            user_input = blank_input.clone();
        } else {
            user_input = temp_input.clone();
        }

        thread::sleep(Duration::from_millis(50));
    };

    clean_up(stdout().by_ref());

    Ok(())
}

/// Listen to the server for messages on a separate thread
fn listen_to_server(stream: &Arc<TcpStream>, sender: Sender<Message>) {
    let mut reader = BufReader::new(stream.as_ref());
    let mut message_type = [0u8; 1];

    loop {
        reader.read_exact(&mut message_type).unwrap();

        match message_type[0] {
            11 => {
                // Game Message
                let mut message = [0u8; 6];
    
                // Read metadata
                reader.read_exact(&mut message).unwrap();
    
                let initial_points = u16::from_le_bytes([message[0], message[1]]);
                let stat_limit = u16::from_le_bytes([message[2], message[3]]);
                let desc_len = u16::from_le_bytes([message[4], message[5]]) as usize;
                let mut desc = vec![0u8; desc_len];
    
                // Read the description
                reader.read_exact(&mut desc).unwrap();
    
                let desc = String::from_utf8(desc).unwrap();
    
                let _ = sender.send(
                    Message::Game {
                        author: stream.clone(),
                        message_type: 11,
                        initial_points,
                        stat_limit,
                        description_len: desc_len as u16,
                        description: desc.into_bytes()
                    }
                );
            },
            14 => {
                // Version Message
                let mut message = [0u8; 4];
    
                // Read metadata
                reader.read_exact(&mut message).unwrap();
    
                let major = u8::from_le_bytes([message[0]]);
                let minor = u8::from_le_bytes([message[1]]);
                let ext_size = u16::from_le_bytes([message[2], message[3]]) as usize;
    
                if ext_size > 0 {
                    let mut ext = vec![0u8; ext_size];
    
                    // Read the extensions
                    reader.read_exact(&mut ext).unwrap();
                }
    
                let _ = sender.send(
                    Message::Version {
                        author: stream.clone(),
                        message_type: 14,
                        major_rev: major,
                        minor_rev: minor,
                        extension_len: ext_size as u16,
                        extensions: Vec::new()
                    }
                );
            }
            _ => {}
        }
    }   
}

/// Push a message to the output buffer and break it up if it is too long (psuedo word wrap)
fn push_to_output(output: &mut Vec<String>, message: String, window: &mut Window) {
    for line in message.lines() {
        // Break up the line if it is too long
        if line.len() > window.w as usize {
            let mut start = 0;
            let mut end = window.w as usize;

            while end < line.len() {
                output.push(line.get(start..end).unwrap().to_string());
                start = end;
                end += window.w as usize;

                window.scroll_ptr += 1;
            }

            // Pad the last line
            let remaining = line.get(start..).unwrap();
            let pad = " ".repeat(window.w as usize - remaining.len());

            output.push(format!("{}{}", remaining, pad));

            window.scroll_ptr += 1;
        } else {
            // Pad
            let pad = " ".repeat(window.w as usize - line.len());
            output.push(format!("{}{}", line, pad));

            window.scroll_ptr += 1;
        }
    }
}

/// Draw the text in the chat window
fn chat_window(stdout: &mut impl Write, chat: &[String], boundary: &Window) {
    let m = chat.len().checked_sub(boundary.h as usize).unwrap_or(0);

    for (i, line) in chat.iter().skip(m).enumerate() {
        stdout.queue(MoveTo(boundary.x, boundary.y + i as u16)).unwrap();

        // Write the line, but only up to the boundary width
        let bytes = line.as_bytes();
        stdout.write(bytes.get(0..boundary.w as usize).unwrap_or(bytes)).unwrap();
    }
}

/// Clean up the terminal
fn clean_up(stdout: &mut impl Write) {
    let _ = terminal::disable_raw_mode().unwrap();
    stdout.queue(Show).unwrap();
    stdout.queue(Clear(ClearType::All)).unwrap();
    stdout.queue(MoveTo(0, 0)).unwrap();

    // Leave alternate screen
    stdout.queue(LeaveAlternateScreen).unwrap();

    stdout.flush().unwrap();
}
