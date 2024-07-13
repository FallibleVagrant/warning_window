use std::io::{self, stdout};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute,
    style::{self, Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    cursor,
    QueueableCommand,
    ExecutableCommand,
};

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

fn update_state(warn_state: &mut WarnStates, window_should_close: &mut bool, rx: &Receiver<Packet>) {
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

    // if is_key_pressed(Key::R) {
    //     *warn_state = WarnStates::None;
    // }

    if packet.is_some() {
        let packet = packet.unwrap();
        if packet.text.is_some() {
            //WARN: text should be sanitized as adhocrays can't handle UTF8 with NULLs in the
            //middle of a string.
            //*text = packet.text.unwrap();
        } else {
            // writeln!(log, "");
        }
        match packet.packet_type {
            PacketType::Warn => *warn_state = WarnStates::Warn,
            PacketType::Alert => *warn_state = WarnStates::Alert,
            _ => (),
        };
    }
}

fn draw(warn_state: &WarnStates) -> io::Result<()> {
    let ascii_width = warn_state.get_ascii_art_width();
    let ascii_height = warn_state.get_ascii_art_height();

    let warn_text = warn_state.get_ascii_art();

    let (cols, rows) = terminal::size()?;

    let ascii_x = (cols / 2) - (ascii_width / 2) as u16;
    let ascii_y = (rows / 2) - (ascii_height / 2) as u16;

    let mut stdout = stdout();

    //Debug information in top left.
    stdout.queue(cursor::MoveTo(0, 0))?
        .queue(style::Print(format!("ascii_x: {}", ascii_x)))?.queue(cursor::MoveToNextLine(1))?
        .queue(style::Print(format!("ascii_y: {}", ascii_y)))?.queue(cursor::MoveToNextLine(1))?
        .queue(style::Print(format!("cols: {}", cols)))?.queue(cursor::MoveToNextLine(1))?
        .queue(style::Print(format!("rows: {}", rows)))?.queue(cursor::MoveToNextLine(1))?;

    stdout.queue(cursor::MoveTo(ascii_x, ascii_y))?;
    let ascii_art = warn_state.get_ascii_art();
    for line in ascii_art.lines() {
        stdout.queue(style::Print(line))?
            .queue(cursor::MoveDown(1))?
            .queue(cursor::MoveToColumn(ascii_x))?;
    }

    stdout.flush()?;

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

fn handle_packet(connection: &mut TcpStream, peer_addr: &str, mut log: Arc<File>) -> Result<Packet, Error> {
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
        writeln!(log, "INFO: Closed connection to {peer_addr}: client disconnected.");
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
        writeln!(log, "INFO: Closed connection to {peer_addr}: num_bytes_in_packet invalid, ({num_bytes_in_packet}).");
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
        writeln!(log, 
            "INFO: Closed connection to {}: num_bytes_in_packet != total_num_bytes_read, ({} != {}).",
            peer_addr,
            num_bytes_in_packet,
            num_bytes_read + 1
        );
        return Err(Error::new(
            ErrorKind::Other,
            "Num of bytes read does not match num of bytes declared in header by client.",
        ));
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

    match packet_type {
        PacketType::Info => {
            if packet_text == None {
                writeln!(log, "INFO: Closed connection to {peer_addr}: sent INFO packet without text.");
                return Err(Error::new(
                    ErrorKind::Other,
                    "Client sent INFO packet without text.",
                ));
            }
            write!(log, "INFO: Received INFO packet from {peer_addr}");
        }
        PacketType::Warn => {
            write!(log, "INFO: Received WARN packet from {peer_addr}");
        }
        PacketType::Alert => {
            write!(log, "INFO: Received ALERT packet from {peer_addr}");
        }
        PacketType::Name => {
            if packet_text == None {
                writeln!(log, "INFO: Closed connection to {peer_addr}: sent NAME packet without text.");
                return Err(Error::new(
                    ErrorKind::Other,
                    "Client sent NAME packet without text.",
                ));
            }
            eprint!("INFO: Recieved NAME packet from {peer_addr}");
        }
    }

    if packet_text.is_some() {
        writeln!(log, " with text: \"{}\".", packet_text.as_deref().unwrap());
    } else {
        writeln!(log, ".");
    }

    return Ok(Packet {
        packet_type: packet_type,
        text: packet_text,
    });
}

fn handle_connection(mut connection: TcpStream, tx: Sender<Packet>, mut log: Arc<File>) {
    //connection_thread handles the particulars of each connection,
    //before sending out data through the channel to the main thread.
    let _connection_thread = thread::spawn(move || {
        //First, associate with the client without allocating state or logging.
        handle_association(&mut connection).unwrap();

        let peer_addr = connection
            .peer_addr()
            .expect("Client is already connected.")
            .to_string();
        writeln!(log, "INFO: Received connection from {peer_addr}.");

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
        stdout().execute(terminal::EnterAlternateScreen);
        stdout().execute(terminal::Clear(terminal::ClearType::All));
        return WindowContext {};
    }
}

impl Drop for WindowContext {
    fn drop(&mut self) {
        stdout().execute(terminal::LeaveAlternateScreen);
    }
}

use std::fs::File;
use std::sync::Arc;

fn main() -> io::Result<()> {
    // env::set_var("RUST_BACKTRACE", "1");
    let mut warn_state = WarnStates::None;
    let mut window_should_close = false;
    let mut log = Arc::new(File::create("./warning_window.log")?);

    //Init the window, clean up on drop.
    let wc = WindowContext::new();

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
                    writeln!(_log, "ERROR: {}", e);
                }
            }
        }
    });

    while !window_should_close {
        if poll(Duration::from_millis(500))? {
            // It's guaranteed that the `read()` won't block when the `poll()`
            // function returns `true`
            match read()? {
                Event::FocusGained => writeln!(log, "FocusGained")?,
                Event::FocusLost => writeln!(log, "FocusLost")?,
                Event::Key(event) => {
                    if let KeyCode::Char(c) = event.code {
                        if c == 'q' {
                            window_should_close = true;
                        }
                        if c == 'c' && event.modifiers == KeyModifiers::CONTROL {
                            window_should_close = true;
                        }
                    }
                    if event.code == KeyCode::Esc {
                        window_should_close = true;
                    }

                    if let KeyCode::Char(c) = event.code {
                        if c == 'r' {
                            warn_state = WarnStates::None;
                        }
                    }

                    writeln!(log, "{:?}", event);
                },
                Event::Resize(width, height) => writeln!(log, "New size {}x{}", width, height)?,
                _ => (),
            }
        } else {
            // Timeout expired and no `Event` is available
            update_state(&mut warn_state, &mut window_should_close, &rx);
            draw(&warn_state)?;
        }
    }

    return Ok(());
}
