use gtk::prelude::*;
use gtk::{
    Application, 
    ApplicationWindow,
    Button,
    Box,
    Entry,
    Label,
    Orientation,
    TextBuffer,
    TextTagTable,
    TextView
};
use regex::Regex;

fn main() {
    let app = Application::builder()
        .application_id("riley.ziegler.LurkSoldier")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let label = Label::builder()
        .label("Connect to Lurk Server")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let button = Button::builder()
        .label("Connect")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let server_input = Entry::builder()
        .placeholder_text("HOSTNAME:PORT")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let command_input = Entry::builder()
        .placeholder_text("")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let text_buf = TextBuffer::new(Some(&TextTagTable::new()));

    let text_view = TextView::builder()
        .buffer(&text_buf)
        .focusable(false)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    text_view.set_size_request(500, 300);

    let content = Box::new(Orientation::Vertical, 0);

    content.append(&label);
    content.append(&server_input);
    content.append(&button);
    content.append(&text_view);
    content.append(&command_input);
    
    // We create the main window.
    let win = ApplicationWindow::builder()
        .title("Lurk Soldier")
        .application(app)
        .child(&content)
        .build();

    button.connect_clicked(move |_| connect(&label, &server_input, &text_buf));

    // Don't forget to make all widgets visible.
    win.show();
}

fn connect(label: &Label, input: &Entry, text_box: &TextBuffer) {
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

    let connected_host: &str = collection[0];
    let connected_port: &str = collection[1];

    // TODO: Connect using host/port via TCP

    // TODO: Update only if we can connect to the server via TCP
    label.set_text(format!("Connected to {}:{}",connected_host, connected_port).as_str());
    text_box.set_text("Welcome to the Thunder Dome!");
}
