use adhocrays::*;

use std::io::stdout;
use std::time::Duration;

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};

enum WarnStates {
    None,
    Warn,
    Alert,
}

impl WarnStates {
    fn get_ascii_art(&self) -> &str {
        match self {
            Self::None => concat!("  / \\  \n", "       \n", "  \\ /  \n",),
            Self::Warn => concat!(
                "       \n",
                "       \n",
                "   O   \n",
                "  /|\\  \n",
                "       \n",
            ),
            Self::Alert => concat!("   .   \n", "  / \\  \n", " / ! \\ \n", "+-----+\n",),
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
            Self::None => Color {
                r: 24,
                g: 24,
                b: 24,
                a: 255,
            },
            Self::Warn => Color {
                r: 244,
                g: 131,
                b: 37,
                a: 255,
            }, //Also try #FF9F43.
            Self::Alert => Color {
                r: 179,
                g: 0,
                b: 0,
                a: 255,
            },
        }
    }
}

use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use std::net::{TcpListener, TcpStream};

use std::sync::mpsc::Receiver;

fn update_state(warn_state: &mut WarnStates, text: &mut String, rx: &Receiver<Packet>) {
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

    if is_key_pressed(Key::R) {
        *warn_state = WarnStates::None;
    }

    if packet.is_some() {
        let packet = packet.unwrap();
        if packet.text.is_some() {
            //WARN: text should be sanitized as adhocrays can't handle UTF8 with NULLs in the
            //middle of a string.
            *text = packet.text.unwrap();
        } else {
            // println!("");
        }
        match packet.packet_type {
            PacketType::Warn => *warn_state = WarnStates::Warn,
            PacketType::Alert => *warn_state = WarnStates::Alert,
            _ => (),
        };
    }
}

fn draw(wc: &WindowContext, warn_state: &WarnStates, text: &str) {
    let mut dc = wc.init_drawing_context();
    dc.clear_background(warn_state.get_color());

    // let ascii_width = warn_state.get_ascii_art_width();
    // let ascii_height = warn_state.get_ascii_art_height();

    let font_size: i32 = 20;

    let warn_text = warn_state.get_ascii_art();
    let ascii_size = measure_text_ex(get_default_font(), warn_text, font_size as f32, 1.5);

    let ascii_width = ascii_size.x as u32;
    let ascii_height = ascii_size.y as u32;

    let ascii_x = (get_screen_width() as u32 / 2) - (ascii_width / 2);
    let ascii_y = (get_screen_height() as u32 / 2) - (ascii_height / 2);

    //Debug information in top left.
    dc.draw_text(
        &format!("ascii_x: {}", ascii_x),
        0,
        font_size * 1,
        font_size,
        colors::LIGHT_GRAY,
    );
    dc.draw_text(
        &format!("ascii_y: {}", ascii_y),
        0,
        font_size * 2,
        font_size,
        colors::LIGHT_GRAY,
    );
    dc.draw_text(
        &format!("mouse_x: {}", get_mouse_position().x),
        0,
        font_size * 3,
        font_size,
        colors::LIGHT_GRAY,
    );
    dc.draw_text(
        &format!("mouse_y: {}", get_mouse_position().y),
        0,
        font_size * 4,
        font_size,
        colors::LIGHT_GRAY,
    );

    dc.draw_text(
        warn_state.get_ascii_art(),
        ascii_x as i32,
        ascii_y as i32,
        20,
        colors::LIGHT_GRAY,
    );

    dc.draw_text(&get_fps().to_string(), 0, 0, 20, colors::LIGHT_GRAY);
    dc.draw_text(text, 200, 50, 20, colors::LIGHT_GRAY);

    //Lines to help center things.
    dc.draw_line_ex(
        Vector2 {
            x: get_screen_width() as f32 / 2.0,
            y: 0.0,
        },
        Vector2 {
            x: get_screen_width() as f32 / 2.0,
            y: get_screen_height() as f32,
        },
        1.0,
        colors::LIGHT_GRAY,
    );
    dc.draw_line_ex(
        Vector2 {
            x: 0.0,
            y: get_screen_height() as f32 / 2.0,
        },
        Vector2 {
            x: get_screen_width() as f32,
            y: get_screen_height() as f32 / 2.0,
        },
        1.0,
        colors::LIGHT_GRAY,
    );
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

fn handle_packet(connection: &mut TcpStream, peer_addr: &str) -> Result<Packet, Error> {
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

    // println!("DEBUG: Received packet from {}.", peer_addr);

    if num_bytes_read == 0 {
        //The other side has closed the connection; terminate the thread.
        println!(
            "INFO: Closed connection to {}: client disconnected.",
            peer_addr
        );
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
        println!(
            "INFO: Closed connection to {}: num_bytes_in_packet invalid, ({}).",
            peer_addr, num_bytes_in_packet
        );
        return Err(Error::new(
            ErrorKind::Other,
            "Invalid number of bytes declared by packet header.",
        ));
    }

    // println!("DEBUG: Packet reports it is {} bytes long.", num_bytes_in_packet);

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

    // println!("DEBUG: Successfully read {} more bytes of the packet.", num_bytes_read);

    //                                 Plus one for the initial byte.
    //                                         v
    if num_bytes_in_packet != num_bytes_read + 1 {
        //TODO: Read may have been interrupted by a signal; try to get the rest of it.
        //For now, close the connection.
        println!("INFO: Closed connection to {}: num_bytes_in_packet != total_num_bytes_read, ({} != {}).", peer_addr, num_bytes_in_packet, num_bytes_read + 1);
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
        // println!("DEBUG: Received text: {} of {} bytes.", packet_text.clone().unwrap(), packet_text.clone().unwrap().len();
    } else {
        packet_text = None;
    }

    match packet_type {
        PacketType::Info => {
            if packet_text == None {
                println!(
                    "INFO: Closed connection to {}: sent INFO packet without text.",
                    peer_addr
                );
                return Err(Error::new(
                    ErrorKind::Other,
                    "Client sent INFO packet without text.",
                ));
            }
            print!("INFO: Received INFO packet from {}", peer_addr);
        }
        PacketType::Warn => {
            print!("INFO: Received WARN packet from {}", peer_addr);
        }
        PacketType::Alert => {
            print!("INFO: Received ALERT packet from {}", peer_addr);
        }
        PacketType::Name => {
            if packet_text == None {
                println!(
                    "INFO: Closed connection to {}: sent NAME packet without text",
                    peer_addr
                );
                return Err(Error::new(
                    ErrorKind::Other,
                    "Client sent NAME packet without text.",
                ));
            }
            print!("INFO: Recieved NAME packet from {}", peer_addr);
        }
    }

    if packet_text.is_some() {
        println!(" with text: \"{}\".", packet_text.as_deref().unwrap());
    } else {
        println!(".");
    }

    return Ok(Packet {
        packet_type: packet_type,
        text: packet_text,
    });
}

fn handle_connection(mut connection: TcpStream, tx: Sender<Packet>) {
    //connection_thread handles the particulars of each connection,
    //before sending out data through the channel to the main thread.
    let _connection_thread = thread::spawn(move || {
        //First, associate with the client without allocating state or logging.
        handle_association(&mut connection).unwrap();

        let peer_addr = connection
            .peer_addr()
            .expect("Client is already connected.")
            .to_string();
        println!("INFO: Received connection from {}.", peer_addr);

        loop {
            //Read exactly one packet from kernel's internal buffer and return it.
            let packet = match handle_packet(&mut connection, &peer_addr) {
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

fn main() {
    // env::set_var("RUST_BACKTRACE", "1");
    let mut warn_state = WarnStates::None;
    let mut text = String::new();

    let wc = init_window_context(800, 450, "warn_window");

    let (tx, rx) = channel::<Packet>();

    //The connection_manager thread lives as long as main.
    //It never exits, and continually handles incoming connections.
    let _connection_manager = thread::spawn(move || {
        let listener = TcpListener::bind("localhost:44444").unwrap();

        for connection in listener.incoming() {
            match connection {
                Ok(c) => handle_connection(c, tx.clone()),
                Err(e) => {
                    eprintln!("ERROR: {}", e);
                }
            }
        }
    });

    while !wc.window_should_close() {
        update_state(&mut warn_state, &mut text, &rx);

        draw(&wc, &warn_state, &text);
    }
}
