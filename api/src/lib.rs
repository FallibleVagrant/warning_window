use std::net::TcpStream;
use std::io::{Read, Write, Error, ErrorKind};

pub struct Session {
    connection: TcpStream,
}

impl Session {
    pub fn connect(addr: &str) -> Result<Session, Error> {
        let mut connection = TcpStream::connect(addr)?;

        //Attempt to associate with the server.
        let mut buf: [u8; 2] = [1, 0];
        let num_bytes_wrote = connection.write(&buf)?;

        if num_bytes_wrote != 2 {
            return Err(Error::new(ErrorKind::Other, "Failed to associate: could not write to server."));
        }

        let num_bytes_read = connection.read(&mut buf)?;

        if num_bytes_read != 2 {
            return Err(Error::new(ErrorKind::Other, "Failed to associate: server did not respond."));
        }

        if buf[0] != 1 && buf[1] != 1 {
            let peer_addr = connection.peer_addr().expect("Client is connected.").to_string();
            println!("Associated with {}.", peer_addr);
        }

        return Ok(Session { connection: connection });
    }

    pub fn send_info(&mut self, msg: &str) -> Result<(), Error> {
        if msg.len() == 0 {
            panic!("INFO messages MUST be non-zero length.");
        }
        self.send(2, msg)
    }

    pub fn send_warn(&mut self, msg: &str) -> Result<(), Error> {
        self.send(3, msg)
    }

    pub fn send_alert(&mut self, msg: &str) -> Result<(), Error> {
        self.send(4, msg)
    }

    fn send(&mut self, packet_type: u8, msg: &str) -> Result<(), Error> {
        let mut buf: [u8; 256] = [0; 256];

        buf[1] = packet_type;

        if msg.len() > 254 {
            return Err(Error::new(ErrorKind::Other, "Message is too long!"));
        }

        //Set num_bytes in packet -- 00000000 means there is 1 byte in packet, 00000001 means there
        //are two bytes, 11111111 means there are 256 bytes, etc.
        //So add num of bytes in msg plus 1 byte for packet_type.
        //Incidentally, num_bytes should never be 00000000 as there is always a packet_type.
        buf[0] = msg.len() as u8 + 1;
        let num_bytes = buf[0] as usize;

        for i in 2..num_bytes + 1 {
            buf[i] = msg.as_bytes()[i - 2];
        }

        // println!("DEBUG: msg {}, len {}, num_bytes {}", msg, msg.len(), num_bytes + 1);

        let num_bytes_wrote = match self.connection.write(&buf[0..num_bytes + 1]) {
            Ok(0) => {
                return Err(Error::from(ErrorKind::UnexpectedEof));
            },
            Ok(n) => {
                n
            },
            Err(e) => {
                return Err(e);
            },
        };

        if num_bytes_wrote != num_bytes + 1 {
            return Err(Error::new(ErrorKind::Other, "Could not write full message to server!"));
        }

        return Ok(());
    }
}
