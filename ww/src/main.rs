use std::io::{self, stdout};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute,
    style::{self, Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    cursor,
    QueueableCommand,
    ExecutableCommand,
    queue,
};

#[derive(Copy, Clone, PartialEq)]
enum WarnStates {
    None,
    Warn,
    Alert,
}

impl WarnStates {
    fn get_ascii_art(&self) -> &str {
        match self {
            Self::None => concat!(
                "  / \\  \n",
                "       \n",
                "  \\ /  \n",
            ),
            Self::Warn => concat!(
                "       \n",
                "       \n",
                "   O   \n",
                "  /|\\  \n",
                "       \n",
            ),
            Self::Alert => concat!(
                "   .   \n",
                "  / \\  \n",
                " / ! \\ \n",
                "+-----+\n",
            ),
        }
    }

    fn get_ascii_art_width(&self) -> u32 {
        return 7;
    }

    fn get_ascii_art_height(&self) -> u32 {
        match self {
            Self::None => 3,
            Self::Warn => 5,
            Self::Alert => 4,
        }
    }

    fn get_color(&self) -> Color {
        match self {
            Self::None => Color::Rgb { r: 24, g: 24, b: 24, },
            Self::Warn => Color::Rgb { r: 244, g: 131, b: 37, }, //Also try #FF9F43.
            Self::Alert => Color::Rgb { r: 179, g: 0, b: 0, },
        }
    }
}

use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use std::net::{TcpListener, TcpStream};

use std::sync::mpsc::Receiver;

fn update(state: &mut State, render_state: &mut RenderState, rx: &Receiver<Packet>) -> io::Result<()> {
    //We have a received a packet when packet is Some.
    //I initially went with a packet_received variable, but the borrow checker complained
    //about borrowing a variable that moved between loops *despite being assigned*.
    let mut packet: Option<Packet> = None;
    match rx.try_recv() {
        Ok(p) => {
            packet = Some(p);
        }
        Err(e) => match e {
            TryRecvError::Empty => (),
            TryRecvError::Disconnected => {
                panic!("Reached an impossible state: connection_manager was closed before main loop finished.");
            }
        },
    }

    //Every 500 ms, we render. If a keypress is received, render immediately.
    if poll(Duration::from_millis(500))? {
        // It's guaranteed that the `read()` won't block when the `poll()`
        // function returns `true`
        match read()? {
            Event::Key(event) => {
                //[q]uit.
                if let KeyCode::Char(c) = event.code {
                    if c == 'q' {
                        state.window_should_close = true;
                    }
                    if c == 'c' && event.modifiers == KeyModifiers::CONTROL {
                        state.window_should_close = true;
                    }
                }
                if event.code == KeyCode::Esc {
                    state.window_should_close = true;
                }

                //Regular keybindings.
                if let KeyCode::Char(c) = event.code {
                    match c {
                        //[r]eset warn state.
                        'r' => {
                            state.warn_state = WarnStates::None
                        },
                        //[f]ocus mode toggle.
                        'f' => {
                            state.is_focused_mode = !state.is_focused_mode;
                            render_state.focused_mode_changed = true;
                        },
                        _ => (),
                    }
                }
            },
            Event::Resize(_width, _height) => {
                // writeln!(log.lock().unwrap(), "New size {}x{}", width, height)?
                *render_state = RenderState::rerender_all();
            },
            _ => (),
        }
    } else {
        // Timeout expired and no `Event` is available
    }

    if packet.is_some() {
        let packet = packet.unwrap();
        if packet.text.is_some() {
            //WARN: text should be sanitized as crossterm probably? can't handle UTF8 with NULLs in the
            //middle of a string.
            //*text = packet.text.unwrap();
        } else {
            // writeln!(log, "");
        }
        match packet.packet_type {
            PacketType::Warn => {
                if state.warn_state != WarnStates::Alert {
                    state.warn_state = WarnStates::Warn
                }
            },
            PacketType::Alert => state.warn_state = WarnStates::Alert,
            _ => (),
        };
    }

    return Ok(());
}

fn render_warn_state(warn_state: WarnStates) -> io::Result<()> {
    let mut stdout = stdout();
    let ascii_width = warn_state.get_ascii_art_width();
    let ascii_height = warn_state.get_ascii_art_height();

    let (cols, rows) = terminal::size()?;

    let ascii_x = (cols / 2) - (ascii_width / 2) as u16;
    let ascii_y = (rows / 2) - (ascii_height / 2) as u16;

    queue!(stdout, cursor::MoveTo(ascii_x, ascii_y), style::SetBackgroundColor(warn_state.get_color()))?;
    let ascii_art = warn_state.get_ascii_art();
    for line in ascii_art.lines() {
        queue!(
            stdout,
            style::Print(line),
            cursor::MoveDown(1),
            cursor::MoveToColumn(ascii_x),
        )?;
    }
    queue!(stdout, style::ResetColor)?;

    return Ok(());
}

fn render(state: &State, render_state: &mut RenderState) -> io::Result<()> {
    let mut stdout = stdout();

    if render_state.clear_background {
        queue!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
        )?;
    }

    //Debug information in top left.
    queue!(
        stdout,
        cursor::MoveTo(0, 0),
        // style::Print(format!("ascii_x: {}", ascii_x)), cursor::MoveToNextLine(1),
        // style::Print(format!("ascii_y: {}", ascii_y)), cursor::MoveToNextLine(1),
        // style::Print(format!("cols: {}", cols)), cursor::MoveToNextLine(1),
        // style::Print(format!("rows: {}", rows)), cursor::MoveToNextLine(1),
    )?;

    //Print the ascii art representing the warn state.
    if render_state.warn_state_changed {
        render_warn_state(state.warn_state)?;
    }

    if render_state.focused_mode_changed {
        if state.is_focused_mode {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("Focus!"))?;
        }
        else {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("      "))?;
        }
    }

    stdout.flush()?;

    //It is implicit that render() will deal with every field in render_state if true,
    //so to avoid manually tracking that we have dealt with everything, we simply create
    //a new render_state where everything is false.
    *render_state = RenderState::new();

    return Ok(());
}

use std::io::{Error, ErrorKind, Read, Write}; //Import the Read, Write traits for TcpStream.
use std::sync::mpsc::Sender;
use std::time::Duration;

fn handle_association(connection: &mut TcpStream) -> Result<(), Error> {
    //Set timeout so connections must associate or be dropped.
    connection
        .set_read_timeout(Some(Duration::from_millis(200)))
        .expect("No errors unless duration is 0.");

    let mut buf: [u8; 2] = [0; 2];
    let num_bytes_read = match connection.read(&mut buf) {
        Ok(0) => {
            //Drop the connection without logging anything - client disconnected for some reason.
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        Ok(n) => n,
        Err(e) => {
            //In the case of any error - whether TimedOut, WouldBlock, even Interrupted - drop the
            //connection. Association is not expensive.
            return Err(e);
        }
    };

    //Okay, we got something from the client.

    if num_bytes_read != 2 {
        //But it must be two bytes! The exact size of the association request.
        //If the client only manages to send one byte they should simply retry association.
        //If they send more the packet isn't an association request.
        return Err(Error::new(
            ErrorKind::Other,
            "Could not associate: received incorrect num of bytes from client.",
        ));
    }

    //Check that it *is* an association request.
    if buf[0] != 1 && buf[1] != 1 {
        return Err(Error::new(
            ErrorKind::Other,
            "Could not associate: packet received from client was not an association request.",
        ));
    }

    //Must send association accept, but timeout if the client suddenly decides to stop ACKing.
    connection
        .set_write_timeout(Some(Duration::from_millis(200)))
        .expect("No errors unless duration is 0.");

    let buf: [u8; 2] = [1, 1];
    let num_bytes_wrote = match connection.write(&buf) {
        Ok(0) => {
            //Drop the connection without logging anything - socket is broken for some reason.
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        Ok(n) => n,
        Err(e) => {
            //In the case of any error - whether TimedOut, WouldBlock, even Interrupted - drop the
            //connection. Association is not expensive.
            return Err(e);
        }
    };

    if num_bytes_wrote != 2 {
        //If the server only manages to send one byte it should simply drop the connection and
        //let the client retry association.
        return Err(Error::new(
            ErrorKind::Other,
            "Could not associate: wrote incorrect num of bytes to client.",
        ));
    }

    //Set back to blocking - associated clients CAN slow loris.
    connection
        .set_read_timeout(None)
        .expect("No errors unless duration is 0.");
    connection
        .set_write_timeout(None)
        .expect("No errors unless duration is 0.");

    //We are associated! We can start receiving data!
    return Ok(());
}

#[derive(Debug)]
enum PacketType {
    Info,
    Warn,
    Alert,
    Name,
}

impl PacketType {
    fn from_type_number(type_number: u8) -> Result<PacketType, Error> {
        match type_number {
            2 => Ok(PacketType::Info),
            3 => Ok(PacketType::Warn),
            4 => Ok(PacketType::Alert),
            5 => Ok(PacketType::Name),
            _ => Err(Error::new(ErrorKind::Other, "Invalid packet type.")),
        }
    }

    fn to_type_number(&self) -> u8 {
        match self {
            PacketType::Info => 2,
            PacketType::Warn => 3,
            PacketType::Alert => 4,
            PacketType::Name => 5,
        }
    }
}

#[derive(Debug)]
struct Packet {
    packet_type: PacketType,
    text: Option<String>,
}

fn handle_packet(connection: &mut TcpStream, peer_addr: &str, log: Arc<Mutex<File>>) -> Result<Packet, Error> {
    //Read exactly one byte from the kernel's read queue. The first byte of every packet is the
    //length of the packet in total bytes. This prevents us from reading multiple packets from the
    //queue at once.
    let mut buf: [u8; 256] = [0; 256];
    let num_bytes_read = match connection.read(&mut buf[0..1]) {
        Ok(0) => 0,
        Ok(n) => n,
        Err(e) => {
            //In the case of any error - whether TimedOut, WouldBlock, even Interrupted - drop the
            //connection.
            //TODO: Make reading packets error-tolerant.
            return Err(e);
        }
    };

    // writeln!(log, "DEBUG: Received packet from {}.", peer_addr);

    if num_bytes_read == 0 {
        //The other side has closed the connection; terminate the thread.
        writeln!(log.lock().unwrap(), "INFO: Closed connection to {peer_addr}: client disconnected.").unwrap();
        return Err(Error::new(
            ErrorKind::Other,
            "Client closed the connection.",
        ));
    }

    //                                          Add one back into num_bytes to get the true number.
    //                                          v
    let num_bytes_in_packet = buf[0] as usize + 1;
    if num_bytes_in_packet == 1 {
        //Ill-formed packet! The client is sending junk! Close the connection.
        //Protocol does not handle single-byte packets.
        //num_bytes_in_packet will never exceed 256, as buf[0] is only a u8.
        writeln!(log.lock().unwrap(), "INFO: Closed connection to {peer_addr}: num_bytes_in_packet invalid, ({num_bytes_in_packet}).").unwrap();
        return Err(Error::new(
            ErrorKind::Other,
            "Invalid number of bytes declared by packet header.",
        ));
    }

    // writeln!(log, "DEBUG: Packet reports it is {} bytes long.", num_bytes_in_packet);

    //Good. We know how large the packet will be. Let's try to read the rest of it.
    let num_bytes_read = match connection.read(&mut buf[1..num_bytes_in_packet]) {
        Ok(0) => 0,
        Ok(n) => n,
        Err(e) => {
            //In the case of any error - whether TimedOut, WouldBlock, even Interrupted - drop the
            //connection.
            //TODO: Make reading packets error-tolerant.
            return Err(e);
        }
    };

    // writeln!(log, "DEBUG: Successfully read {} more bytes of the packet.", num_bytes_read);

    //                                 Plus one for the initial byte.
    //                                         v
    if num_bytes_in_packet != num_bytes_read + 1 {
        //TODO: Read may have been interrupted by a signal; try to get the rest of it.
        //For now, close the connection.
        writeln!(log.lock().unwrap(),
            "INFO: Closed connection to {}: num_bytes_in_packet != total_num_bytes_read, ({} != {}).",
            peer_addr,
            num_bytes_in_packet,
            num_bytes_read + 1
        ).unwrap();
        return Err(Error::new(ErrorKind::Other, "Num of bytes read does not match num of bytes declared in header by client."));
    }

    let packet_type_number = buf[1];
    let packet_type = PacketType::from_type_number(packet_type_number)?;

    let packet_text: Option<String>;
    //If the packet is longer than two bytes there is optional text.
    //Move this section into a match statement if the protocol expands to have more than optional text
    //fields.
    if num_bytes_in_packet - 2 > 0 {
        packet_text = Some(String::from_utf8_lossy(&buf[2..num_bytes_in_packet]).to_string());
        // writeln!(log, "DEBUG: Received text: {} of {} bytes.", packet_text.clone().unwrap(), packet_text.clone().unwrap().len();
    } else {
        packet_text = None;
    }

    let mut _log = log.lock().unwrap();
    match packet_type {
        PacketType::Info => {
            if packet_text == None {
                writeln!(_log, "INFO: Closed connection to {peer_addr}: sent INFO packet without text.").unwrap();
                return Err(Error::new(ErrorKind::Other, "Client sent INFO packet without text."));
            }
            write!(_log, "INFO: Received INFO packet from {peer_addr}").unwrap();
        }
        PacketType::Warn => {
            write!(_log, "INFO: Received WARN packet from {peer_addr}").unwrap();
        }
        PacketType::Alert => {
            write!(_log, "INFO: Received ALERT packet from {peer_addr}").unwrap();
        }
        PacketType::Name => {
            if packet_text == None {
                writeln!(_log, "INFO: Closed connection to {peer_addr}: sent NAME packet without text.").unwrap();
                return Err(Error::new(
                    ErrorKind::Other,
                    "Client sent NAME packet without text.",
                ));
            }
            write!(_log, "INFO: Recieved NAME packet from {peer_addr}").unwrap();
        }
    }

    if packet_text.is_some() {
        writeln!(_log, " with text: \"{}\".", packet_text.as_deref().unwrap()).unwrap();
    } else {
        writeln!(_log, ".").unwrap();
    }

    return Ok(Packet {
        packet_type: packet_type,
        text: packet_text,
    });
}

fn handle_connection(mut connection: TcpStream, tx: Sender<Packet>, log: Arc<Mutex<File>>) {
    //connection_thread handles the particulars of each connection,
    //before sending out data through the channel to the main thread.
    let _connection_thread = thread::spawn(move || {
        //First, associate with the client without allocating state or logging.
        handle_association(&mut connection).unwrap();

        let peer_addr = connection
            .peer_addr()
            .expect("Client is already connected.")
            .to_string();
        writeln!(log.lock().unwrap(), "INFO: Received connection from {peer_addr}.").unwrap();

        loop {
            //Read exactly one packet from kernel's internal buffer and return it.
            let packet = match handle_packet(&mut connection, &peer_addr, Arc::clone(&log)) {
                Ok(p) => Some(p),
                Err(_) => None,
            };

            //Send structured data from packet to main thread.
            if packet.is_some() {
                tx.send(packet.unwrap()).expect("Unable to send on channel");
            } else {
                return;
            }
        }
    });
}

//The protocol:
//
//HEADER:
//[u8][u8]
//  ^   ^----------------------------\
//  |                                |
//num_bytes (inclusive)          packet type
//
//NOTE: num_bytes is mapped to be one less than it actually is, i.e.
//if num_bytes is 00000000, the true num_bytes is 1.
//This is so that 11111111 represents 256 instead of 255.
//You may conceptualize it as the number of bytes that follow the initial one.
//A num_bytes of 00000000 is invalid, as there must be a packet type.
//
//The payload is optional, and depends on the packet type.
//
//PACKET TYPES:
//00000000 - ASSOCIATION REQUEST
//00000001 - ASSOCIATION ACCEPT
//00000010 - CLIENT INFO - text payload
//00000011 - CLIENT WARN - optional text payload
//00000100 - CLIENT ALERT - optional text payload
//00000101 - CLIENT NAME CHANGE - text payload

// use std::env;

struct WindowContext {}

impl WindowContext {
    fn new() -> WindowContext {
        terminal::enable_raw_mode().unwrap();
        execute!(stdout(), terminal::EnterAlternateScreen).unwrap();
        execute!(stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
        execute!(stdout(), cursor::Hide).unwrap();
        return WindowContext {};
    }
}

impl Drop for WindowContext {
    fn drop(&mut self) {
        execute!(stdout(), terminal::LeaveAlternateScreen).unwrap();
    }
}

struct State {
    warn_state: WarnStates,
    window_should_close: bool,
    packet_log: VecDeque<Packet>,

    is_focused_mode: bool,
}

struct RenderState {
    focused_mode_changed: bool,
    warn_state_changed: bool,

    //For when everything needs to be rerendered e.g. on resize.
    clear_background: bool,
}

impl RenderState {
    fn new() -> Self {
        return RenderState {
            focused_mode_changed: false,
            warn_state_changed: false,
            clear_background: false,
        };
    }

    fn rerender_all() -> Self {
        return RenderState {
            focused_mode_changed: true,
            warn_state_changed: true,
            clear_background: true,
        };
    }
}

use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;

fn main() -> io::Result<()> {
    // env::set_var("RUST_BACKTRACE", "1");
    let mut state = State {
        warn_state: WarnStates::None,
        window_should_close: false,
        packet_log: VecDeque::new(),

        is_focused_mode: false,
    };
    let mut render_state = RenderState::rerender_all();

    let log = Arc::new(Mutex::new(File::create("./warning_window.log")?));

    //Init the window, clean up on drop.
    let _wc = WindowContext::new();

    let (tx, rx) = channel::<Packet>();
    let mut _log = Arc::clone(&log);

    //The connection_manager thread lives as long as main.
    //It never exits, and continually handles incoming connections.
    let _connection_manager = thread::spawn(move || {
        let listener = TcpListener::bind("localhost:44444").unwrap();

        for connection in listener.incoming() {
            let mut __log = Arc::clone(&_log);
            match connection {
                Ok(c) => handle_connection(c, tx.clone(), __log),
                Err(e) => {
                    writeln!(_log.lock().unwrap(), "ERROR: {}", e).unwrap();
                }
            }
        }
    });

    while !state.window_should_close {
        //update() will poll for keypresses -- if there are none it continues after 500 ms.
        update(&mut state, &mut render_state, &rx)?;
        //Always render -- after 500 ms or when a key is pressed.
        render(&state, &mut render_state)?;
    }

    return Ok(());
}
