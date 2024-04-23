use gtk::{prelude::*, TextIter};
use std::io::{BufReader, Read};
use gtk::{
    Application, 
    ApplicationWindow,
    Button,
    Box,
    Entry,
    Label,
    Orientation,
    TextBuffer,
    TextView,
    TextTagTable,
    ScrollablePolicy
};

use std::fs::File;
use regex::Regex;
use std::net::TcpStream;

type Result<T> = std::result::Result<T, ()>;

fn main() {
    let app = Application::builder()
        .application_id("riley.ziegler.LurkKnight")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // Main Widget
    let parent = Box::new(Orientation::Horizontal, 2);

    // Connect Area
    let connect_area = Box::new(Orientation::Horizontal, 2);

    // Connect Button
    let connect_button = Button::builder()
        .label("Connect")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Server Input
    let server_input = Entry::builder()
        .placeholder_text("HOSTNAME:PORT")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .hexpand(true)
        .build();

    connect_area.append(&server_input);
    connect_area.append(&connect_button);

    // Main Text Area
    let main_text_area = Box::new(Orientation::Vertical, 2);
   
    // Main Label
    let title = Label::builder()
        .label("Connect to Lurk Server")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Main Text Area
    let main_table = TextTagTable::new();
/*
    let font_tag = TextTag::builder()
        .name("font")
        .font("Consolas")
        .build();

    main_table.add(&font_tag);
*/
    let main_text_buf = TextBuffer::new(Some(&main_table));

    let text_view = TextView::builder()
        .buffer(&main_text_buf)
        .focusable(false)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .vscroll_policy(ScrollablePolicy::Minimum)
        .build();

    text_view.set_editable(false);
    text_view.set_size_request(1080, 700);

    // Command Input
    let command_input = Entry::builder()
        .placeholder_text("")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    main_text_area.append(&title);
    main_text_area.append(&connect_area);
    main_text_area.append(&text_view);
    main_text_area.append(&command_input);

    // Side Panel
    let side_panel = Box::new(Orientation::Vertical, 2);

    // Player/ Monster Label
    let player_monster_label = Label::builder()
        .label("Players/Monsters")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Player/ Monster List
    let mon_table = TextTagTable::new();
    let mon_text_buf = TextBuffer::new(Some(&mon_table));

    let player_text_view = TextView::builder()
        .buffer(&mon_text_buf)
        .focusable(false)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .vexpand(true)
        .build();

    // Stats Label
    let stats_label = Label::builder()
        .label("Stats")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Stats List
    let stats_table = TextTagTable::new();
    let stats_text_buf = TextBuffer::new(Some(&stats_table));

    let stat_text_view = TextView::builder()
        .buffer(&stats_text_buf)
        .focusable(false)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .vexpand(true)
        .build();

    side_panel.append(&player_monster_label);
    side_panel.append(&player_text_view);
    side_panel.append(&stats_label);
    side_panel.append(&stat_text_view);

    parent.append(&main_text_area);
    parent.append(&side_panel);

    // Create the main window.
    let win = ApplicationWindow::builder()
        .title("Lurk Knight Client")
        .application(app)
        .default_width(1500)
        .child(&parent)
        .build();

    // Load Lurk Knight Logo
    let mut file = File::open("/home/rjziegler/lurk-server/logo.txt").expect("Unable to opne logo text file.");

    let mut description = String::new();

    file.read_to_string(&mut description).expect("Unable to read the file.");

    main_text_buf.set_text(description.as_str());

    connect_button.connect_clicked(move |button| connect(&title, &server_input, &button, &main_text_buf));

    // Don't forget to make all widgets visible.
    win.show();
}

fn connect(label: &Label, input: &Entry, button: &Button, text_box: &TextBuffer) {
    let re = Regex::new(r"^([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$").unwrap();
    let binding = input.text();
    let parts = binding.split(':');

    let collection = parts.collect::<Vec<&str>>();

    // Validate HostName and Port
    if collection.len() != 2 { 
        label.set_text("Error: Please enter a new host!");
        text_box.set_text("");
        return; 
    }

    if !re.is_match(collection[1]) {
        label.set_text("Port must be a valid port number.");
        text_box.set_text("");
        return;
    }

    // Parse HostName and Port
    let connected_host: &str = collection[0];
    let connected_port: &str = collection[1];

    // Connect using host/port via TCP
    let address = format!("{}:{}", connected_host, connected_port);

    match TcpStream::connect(address) {
        Ok(stream) => {
            println!("Connected to {}:{}", connected_host, connected_port);
            label.set_text(format!("Connected to {}:{}",connected_host, connected_port).as_str());

            // Disable the input field and button
            button.set_sensitive(false);
            input.set_editable(false);
            
            listen_thread(stream, &text_box).expect("Error: Could not start listen thread.");
        },
        Err(e) => {
            eprintln!("{}",e);
            label.set_text(format!("Error: Could not connect to {}:{}", connected_host, connected_port).as_str());
        }
    };
}

// FIXME: If the loop doesnt break, the program will hang
fn listen_thread(stream: TcpStream, text_box: &TextBuffer) -> Result<()> {
    let mut buffer =  BufReader::new(stream);
    let mut message_type = [0u8];
    
    let mut iter: TextIter;

    // Receive messages from the server
    loop {
        buffer.read_exact(&mut message_type).map_err(|err| {
            eprintln!("Error: Could not read message type: {}", err);
        })?;

        iter = text_box.end_iter();

        match message_type[0] {
            11 => {
                println!("Received message type 11");

                let mut metadata = [0u8; 6];

                buffer.read_exact(&mut metadata).map_err(|err| {
                    eprintln!("Error: Could not read metadata: {}", err);
                })?;

                let intial_points = u16::from_le_bytes([metadata[0], metadata[1]]);
                let stat_limit = u16::from_le_bytes([metadata[2], metadata[3]]);
                let desc_len = u16::from_le_bytes([metadata[4], metadata[5]]);

                println!("Initial Points: {}", intial_points);
                println!("Stat Limit: {}", stat_limit);
                println!("Description length: {}", desc_len);

                // If there is no description, continue
                if desc_len == 0 { continue; }

                // Read the description
                let mut description = vec![0u8; desc_len as usize];

                buffer.read_exact(&mut description).map_err(|err| {
                    eprintln!("Error: Could not read description: {}", err);
                })?;

                let desc = String::from_utf8(description).map_err(|err| {
                    eprintln!("Error: Could not parse description: {}", err);
                })?;

                println!("Description: {}", desc);

                text_box.insert(&mut iter, format!("\nInitial Points: {}\tStat Limit: {}\tDescription: {}\n", intial_points, stat_limit, desc).as_str());

                break;
            },
            14 => {
                println!("Received message type 14");

                let mut metadata = [0u8; 4];

                buffer.read_exact(&mut metadata).map_err(|err| {
                    eprintln!("Error: Could not read metadata: {}", err);
                })?;

                let ext_len = u16::from_be_bytes([metadata[2], metadata[3]]);

                println!("Lurk Server Version {}.{} with {} extensions.", metadata[0], metadata[1], ext_len);
                text_box.insert(&mut iter, format!("\nLurk Server Version {}.{} with {} extensions.\n", metadata[0], metadata[1], ext_len).as_str());

                // If there are no extensions, continue
                if ext_len == 0 { continue; }

                // Read the extensions
                let mut extension = vec![0u8; ext_len as usize];

                buffer.read_exact(&mut extension).map_err(|err| {
                    eprintln!("Error: Could not read extensions: {}", err);
                })?;

                let ext = String::from_utf8(extension).map_err(|err| {
                    eprintln!("Error: Could not parse extensions: {}", err);
                })?;

                println!("Extensions: {}", ext);
            },
            _ => {
                println!("Unknown message type {}", message_type[0]);
            }
        }
    }

    Ok(())
}