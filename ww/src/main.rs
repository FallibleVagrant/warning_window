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
    fn to_string(&self) -> &str {
        match self {
            Self::None => "NONE",
            Self::Warn => "WARN",
            Self::Alert => "ALERT",
        }
    }
}

struct WarnStateAsciiArt {
    info_art: String,
    warn_art: String,
    alert_art: String,

    info_color: style::Color,
    warn_color: style::Color,
    alert_color: style::Color,
}

impl WarnStateAsciiArt {
    fn default_info_art() -> String {
        return concat!(
            "  / \\  \n",
            "       \n",
            "  \\ /  \n",
        ).to_string();
    }

    fn default_warn_art() -> String {
        return concat!(
            "       \n",
            "       \n",
            "   O   \n",
            "  /|\\  \n",
            "       \n",
        ).to_string();
    }

    fn default_alert_art() -> String {
        return concat!(
            "   .   \n",
            "  / \\  \n",
            " / ! \\ \n",
            "+-----+\n",
        ).to_string();
    }

    fn new() -> Self {
        return WarnStateAsciiArt {
            info_art: Self::default_info_art(),
            warn_art: Self::default_warn_art(),
            alert_art: Self::default_alert_art(),

            info_color: Color::Rgb { r: 24, g: 24, b: 24, },
            warn_color: Color::Rgb { r: 244, g: 131, b: 37, }, //Also try #FF9F43.
            alert_color: Color::Rgb { r: 179, g: 0, b: 0, },
        };
    }

    fn build(mut info_art: String, mut warn_art: String, mut alert_art: String) -> Self {
        if info_art == "" {
            info_art = Self::default_info_art();
        }
        if warn_art == "" {
            warn_art = Self::default_warn_art();
        }
        if alert_art == "" {
            alert_art = Self::default_alert_art();
        }
        return WarnStateAsciiArt {
            info_art: info_art,
            warn_art: warn_art,
            alert_art: alert_art,

            info_color: Color::Rgb { r: 24, g: 24, b: 24, },
            warn_color: Color::Rgb { r: 244, g: 131, b: 37, }, //Also try #FF9F43.
            alert_color: Color::Rgb { r: 179, g: 0, b: 0, },
        };
    }

    fn build_with_color(mut info_art: String, mut warn_art: String, mut alert_art: String, info_color: style::Color, warn_color: style::Color, alert_color: style::Color) -> Self {
        if info_art == "" {
            info_art = Self::default_info_art();
        }
        if warn_art == "" {
            warn_art = Self::default_warn_art();
        }
        if alert_art == "" {
            alert_art = Self::default_alert_art();
        }
        return WarnStateAsciiArt {
            info_art: info_art,
            warn_art: warn_art,
            alert_art: alert_art,

            info_color,
            warn_color,
            alert_color,
        };
    }

    fn to_ascii_art(&self, warn_state: &WarnStates) -> &str {
        return match warn_state {
            WarnStates::None => &self.info_art,
            WarnStates::Warn => &self.warn_art,
            WarnStates::Alert => &self.alert_art,
        };
    }

    fn width(&self, warn_state: &WarnStates) -> usize {
        match warn_state {
            WarnStates::None => {
                //These return the index of the first \n, or the len().
                return self.info_art.char_indices()                 //        ignore this, just for returning a tuple.
                                    .find(|c| { c.1 == '\n' })      //        v
                                    .unwrap_or_else(|| (self.info_art.len(), 'i')).0;
            },
            WarnStates::Warn => {
                return self.warn_art.char_indices()
                                    .find(|c| { c.1 == '\n' })
                                    .unwrap_or_else(|| (self.warn_art.len(), 'i')).0;
            },
            WarnStates::Alert => {
                return self.alert_art.char_indices()
                                    .find(|c| { c.1 == '\n' })
                                    .unwrap_or_else(|| (self.alert_art.len(), 'i')).0;
            },
        };
    }

    fn height(&self, warn_state: &WarnStates) -> usize {
        match warn_state {
            WarnStates::None => {
                return self.info_art.lines().count();
            },
            WarnStates::Warn => {
                return self.warn_art.lines().count();
            },
            WarnStates::Alert => {
                return self.alert_art.lines().count();
            },
        }
    }

    fn max_width(&self) -> usize {
        return std::cmp::max(
            self.width(&WarnStates::None),
            std::cmp::max(
                self.width(&WarnStates::Warn),
                self.width(&WarnStates::Alert)
            )
        );
    }

    fn max_height(&self) -> usize {
        return std::cmp::max(
            self.height(&WarnStates::None),
            std::cmp::max(
                self.height(&WarnStates::Warn),
                self.height(&WarnStates::Alert)
            )
        );
    }

    fn color(&self, warn_state: &WarnStates) -> Color {
        match warn_state {
            WarnStates::None => {
                return self.info_color;
            },
            WarnStates::Warn => {
                return self.warn_color;
            },
            WarnStates::Alert => {
                return self.alert_color;
            },
        }
    }
}

use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use std::net::{TcpListener, TcpStream, IpAddr, SocketAddr};

use std::sync::mpsc::Receiver;

fn update(state: &mut State, render_state: &mut RenderState, rx: &Receiver<LogItem>, log: Arc<Mutex<File>>) -> io::Result<()> {
    //We have a received a packet when log_item is Some, or otherwise a connection notification
    //from the connecting/disconnecting client.
    //I initially went with a packet_received variable, but the borrow checker complained
    //about borrowing a variable that moved between loops *despite being assigned*.
    let mut log_item: Option<LogItem> = None;
    match rx.try_recv() {
        Ok(l) => {
            log_item = Some(l);
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
                            state.warn_state = WarnStates::None;
                            render_state.warn_state_changed = true;
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

    if log_item.is_some() {
        let log_item = log_item.unwrap();

        match &log_item {
            LogItem::PacketLogItem { peer_addr, packet, .. } => {
                match packet.packet_type {
                    PacketType::Warn => {
                        if state.warn_state != WarnStates::Alert {
                            state.warn_state = WarnStates::Warn;
                            render_state.warn_state_changed = true;
                        }
                    },
                    PacketType::Alert => {
                        state.warn_state = WarnStates::Alert;
                        render_state.warn_state_changed = true;
                    },
                    PacketType::Name => {
                        if packet.text.is_some() {
                            let name = packet.text.as_ref().unwrap();
                            if name.len() < 25 {
                                state.peer_names.insert(*peer_addr, name.clone());
                            }
                        }
                    },
                    _ => (),
                };
            },
            LogItem::DisconnectLogItem { peer_addr, .. } => {
                state.peer_names.remove(peer_addr);
            },
            _ => (),
        }

        state.packet_log.push_front(log_item);
        render_state.packet_log_changed = true;
    }

    return Ok(());
}

fn get_rand_char(rand: usize) -> char {
    return match rand {
        0 => '#',
        1 => '&',
        2 => '+',
        3 => '=',
        4 => '*',
        5 => '-',
        _ => ' ',
    };
}

fn render_alert_border(frame_number: usize, warn_art: &WarnStateAsciiArt) -> io::Result<()> {
    let mut stdout = stdout();
    let (cols, rows) = terminal::size()?;

    //Blank out the border every frame.
    for y in 0..rows {
        let xs: [u16; 8] = [0, 1, 2, 3, cols-4, cols-3, cols-2, cols-1];
        for x in xs {
            queue!(stdout, cursor::MoveTo(x, y), style::Print(' '))?;
        }
    }

    // queue!(stdout, style::SetForegroundColor(warn_art.color(&WarnStates::Alert)))?;
    for y in 0..rows {
        let i = y as usize;

        //Print the streams of characters that appear on the left.
        if true {
            let mut c = get_rand_char((frame_number - i) % 11);
            if (frame_number - i) % 143 <= 80 {
                queue!(stdout, cursor::MoveTo(0, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 9);
            if (frame_number - i) % 223 <= 100 {
                queue!(stdout, cursor::MoveTo(1, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 7);
            if (frame_number - i) % 349 <= 180 {
                queue!(stdout, cursor::MoveTo(2, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 12);
            if (frame_number - i) % 943 <= 200 {
                queue!(stdout, cursor::MoveTo(3, y), style::Print(c))?;
            }
        }

        //Print the streams of characters that appear on the right.
        if true {
            let mut c = get_rand_char((frame_number - i) % 11);
            if (frame_number - i) % 139 <= 90 {
                queue!(stdout, cursor::MoveTo(cols, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 9);
            if (frame_number - i) % 226 <= 130 {
                queue!(stdout, cursor::MoveTo(cols - 1, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 7);
            if (frame_number - i) % 363 <= 200 {
                queue!(stdout, cursor::MoveTo(cols - 2, y), style::Print(c))?;
            }
            c = get_rand_char((frame_number - i) % 12);
            if (frame_number - i) % 927 <= 200 {
                queue!(stdout, cursor::MoveTo(cols - 3, y), style::Print(c))?;
            }
        }

        //Print the bordering '|' characters on the left and right.
        if (frame_number + i) % 6 < 3 {
            queue!(stdout, cursor::MoveTo(0, y), style::Print("|"))?;
            queue!(stdout, cursor::MoveTo(cols, y), style::Print("|"))?;
        }
        if frame_number % 13 + i % 5 <= 3 {
            queue!(stdout, cursor::MoveTo(0, y), style::Print(":"))?;
            queue!(stdout, cursor::MoveTo(cols, y), style::Print(":"))?;
        }
    }
    // queue!(stdout, style::ResetColor)?;

    return Ok(());
}

fn render_warn_state(warn_art: &WarnStateAsciiArt, warn_state: &WarnStates, is_centered: bool, frame_number: usize) -> io::Result<()> {
    let mut stdout = stdout();
    let ascii_width = warn_art.width(warn_state);
    let ascii_height = warn_art.height(warn_state);

    let (cols, rows) = terminal::size()?;

    let ascii_x;
    let ascii_y;
    if is_centered {
        ascii_x = (cols / 2) - (ascii_width / 2) as u16;
        ascii_y = (rows / 2) - (ascii_height / 2) as u16;
    }
    else {
        ascii_x = (cols / 2) - (ascii_width / 2) as u16;
        ascii_y = rows / 5;
    }

    let ascii_min_x;
    let ascii_min_y;
    if is_centered {
        ascii_min_x = (cols / 2) - (warn_art.max_width() / 2) as u16;
        ascii_min_y = (rows / 2) - (warn_art.max_height() / 2) as u16;
    }
    else {
        ascii_min_x = (cols / 2) - (warn_art.max_width() / 2) as u16;
        ascii_min_y = rows / 5;
    }

    //Blank the previous warn_state.
    //These max_glitch variables actually denote the max + 1 due to being used in a modulo.
    //Apologies for any confusion this may cause.
    let max_horizontal_glitch: u16 = 4;
    let max_vertical_glitch: u16 = 3;
    queue!(stdout, cursor::MoveTo(ascii_min_x - max_horizontal_glitch, ascii_min_y - max_vertical_glitch))?;
    for _y in 0..(warn_art.max_height() + (2 * max_vertical_glitch) as usize - 1) {
        for _x in 0..(warn_art.max_width() + (2 * max_horizontal_glitch) as usize - 1) {
            queue!(stdout, style::Print(' '))?;
        }
        queue!(stdout, cursor::MoveDown(1), cursor::MoveToColumn(ascii_min_x - max_horizontal_glitch))?;
    }
    let max_horizontal_glitch: usize = max_horizontal_glitch as usize;
    let max_vertical_glitch: usize = max_vertical_glitch as usize;

    //Print the current warn_state.
    queue!(stdout, cursor::MoveTo(ascii_x, ascii_y), style::SetBackgroundColor(warn_art.color(warn_state)))?;
    let ascii_art = warn_art.to_ascii_art(warn_state);
    for (i, line) in ascii_art.lines().enumerate() {
        //Compute the horizontal and vertical shift applied to the frame every so often.
        let mut horizontal_glitch: i32 = 0;
        if (frame_number + ((i << 3) % 5)) % 284 <= 37 {
            horizontal_glitch = ((((frame_number + i) << 3) % 213) % max_horizontal_glitch) as i32;
            if ((frame_number << 4) + ((i << 5) % 23)) % 2 == 0 {
                horizontal_glitch *= -1;
            }
        }
        let mut vertical_glitch: i32 = 0;
        if (frame_number + ((i << 2) % 7)) % 361 <= 25 {
            vertical_glitch = ((((frame_number + i) << 3) % 213) % max_vertical_glitch) as i32;
            if ((frame_number << 5) + ((i << 4) % 49)) % 2 == 0 {
                vertical_glitch *= -1;
            }
        }
        let x = ascii_x as i32 + horizontal_glitch;
        let y = ascii_y as i32 + i as i32 + vertical_glitch;
        queue!(stdout, cursor::MoveTo(x as u16, y as u16), style::Print(line))?;

        //Original code to print without glitching.
        // queue!(
        //     stdout,
        //     style::Print(line),
        //     cursor::MoveDown(1),
        //     cursor::MoveToColumn(ascii_x),
        // )?;
    }
    queue!(stdout, style::ResetColor)?;

    return Ok(());
}

fn render_packet_log(packet_log: &VecDeque<LogItem>, warn_art_max_height: usize, peer_names: &HashMap<SocketAddr, String>) -> io::Result<()> {
    let mut stdout = stdout();

    let (cols, rows) = terminal::size()?;

    let margin_x = 4;
    let start_x = margin_x as u16;
    let start_y = 2 + warn_art_max_height as u16 + rows / 5;

    //Blank the packet log.
    queue!(stdout, cursor::MoveTo(start_x, start_y))?;
    for _y in start_y..=(rows - 3) {
        for _x in margin_x..=(cols - margin_x) {
            queue!(stdout, style::Print(' '))?;
        }
        queue!(stdout, cursor::MoveDown(1), cursor::MoveToColumn(start_x))?;
    }

    // println!("packet_log len: {}", packet_log.len());
    queue!(stdout, cursor::MoveTo(start_x, start_y))?;
    for log_item in packet_log {
        let timestamp_in_secs = log_item.timestamp().duration_since(UNIX_EPOCH).expect("Time went backwards.").as_secs();

        let secs_per_day  =  24 * 60 * 60;
        let secs_per_hour =  60 * 60;
        let secs_per_min  =  60;

        let hour = (timestamp_in_secs % secs_per_day) / secs_per_hour;
        let min = (timestamp_in_secs % secs_per_hour) / secs_per_min;

        //Print the time.
        queue!(stdout,
            style::Print(
                format!("[{:0>2}:{:0>2}] ", hour, min)
            )
        )?;

        let mut y;

        //Depending on the packet, print different things.
        match &log_item {
            LogItem::ConnectLogItem { peer_addr, .. } => {
                queue!(stdout,
                    style::Print(
                        format!("{} has successfully associated.", peer_addr.to_string())
                    )
                )?;
                queue!(
                    stdout,
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(start_x),
                )?;

                (_, y) = cursor::position().unwrap();
            },
            LogItem::DisconnectLogItem { peer_addr, .. } => {
                queue!(stdout,
                    style::Print(
                        format!("{} has disconnected.", peer_addr.to_string())
                    )
                )?;
                queue!(
                    stdout,
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(start_x),
                )?;

                (_, y) = cursor::position().unwrap();
            },
            LogItem::PacketLogItem { peer_addr, packet, .. } => {
                //Print the packet type.
                queue!(stdout,
                    style::Print(
                        format!("{} | ", packet.packet_type.to_string())
                    )
                )?;

                //Print the peer address/name.
                //Look at this abomination. Rust, please.

                let mut use_name = false;
                let peer_name: &str;
                //NAME packets always print the IP.
                if let PacketType::Name = packet.packet_type {
                    //Negation of if let statements not implemented yet.
                }
                else {
                    let peer_name_option = peer_names.get(peer_addr);
                    if peer_name_option.is_some() {
                        use_name = true;
                        peer_name = peer_name_option.unwrap();
                        queue!(stdout,
                            style::Print(
                                format!("{} | ", peer_name)
                            )
                        )?;
                    }
                }

                if !use_name {
                    queue!(stdout,
                        style::Print(
                            format!("{} | ", peer_addr.to_string())
                        )
                    )?;
                }

                //Print the message text.
                let default = "".to_string();
                let msg = packet.text.as_ref().unwrap_or(&default).as_str();
                let mut x;
                (x, y) = cursor::position().unwrap();
                for c in msg.chars() {
                    if x >= cols - margin_x {
                        if y > rows - 4 {
                            break;
                        }
                        queue!(
                            stdout,
                            cursor::MoveDown(1),
                            cursor::MoveToColumn(start_x),
                        )?;
                        x = start_x;
                        y += 1;
                    }
                    queue!(stdout, style::Print(c))?;
                    x += 1;
                }
                queue!(
                    stdout,
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(start_x),
                )?;
                y += 1;
            },
        }

        //Stop near the bottom of the screen.
        if y > rows - 3 {
            break;
        }
    }
    queue!(stdout, style::ResetColor)?;

    return Ok(());
}

fn render(state: &State, render_state: &mut RenderState, log: Arc<Mutex<File>>, frame_number: usize) -> io::Result<()> {
    let mut stdout = stdout();

    let (cols, rows) = terminal::size()?;
    let min_cols = state.warn_state_ascii_art.width(&state.warn_state) as u16 + 10;
    let min_rows = state.warn_state_ascii_art.height(&state.warn_state) as u16 + 10;
    if cols < min_cols || rows < min_rows {
        writeln!(log.lock().unwrap(), "ERROR: ascii art is too large to render on terminal.").unwrap();
        return Err(Error::new(
            ErrorKind::Other,
            "ERROR: ascii art is too large to render on terminal.",
        ));
    }

    if render_state.clear_background {
        queue!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
        )?;
    }

    //Print the ascii art representing the warn state.
    render_warn_state(&state.warn_state_ascii_art, &state.warn_state, false, frame_number)?;

    //Print the border art when alert.
    if state.warn_state == WarnStates::Alert {
        render_alert_border(frame_number, &state.warn_state_ascii_art)?;
    }
    else {
        //Blank out the border if we have changed away from alert state.
        //Unfortunately, this code also triggers when switching from NONE to WARN.
        if render_state.warn_state_changed {
            //Blank out the border.
            for y in 0..rows {
                let xs: [u16; 8] = [0, 1, 2, 3, cols-4, cols-3, cols-2, cols-1];
                for x in xs {
                    queue!(stdout, cursor::MoveTo(x, y), style::Print(' '))?;
                }
            }
        }
    }

    if render_state.focused_mode_changed {
        if state.is_focused_mode {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("Focus!"))?;
        }
        else {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("      "))?;
        }
    }

    if render_state.packet_log_changed {
        render_packet_log(&state.packet_log, state.warn_state_ascii_art.max_height(), &state.peer_names)?;
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

#[derive(Debug, Copy, Clone)]
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

    fn to_string(&self) -> &str {
        match self {
            PacketType::Info => "INFO",
            PacketType::Warn => "WARN",
            PacketType::Alert => "ALERT",
            PacketType::Name => "NAME",
        }
    }
}

#[derive(Debug, Clone)]
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

fn handle_connection(mut connection: TcpStream, tx: Sender<LogItem>, log: Arc<Mutex<File>>) {
    //connection_thread handles the particulars of each connection,
    //before sending out data through the channel to the main thread.
    let _connection_thread = thread::spawn(move || {
        //First, associate with the client without allocating state or logging.
        handle_association(&mut connection).unwrap();

        let peer_addr = connection
            .peer_addr()
            .expect("Client is already connected.");
        let peer_addr_str = peer_addr.to_string();

        //Send a connection notice to the packet_log.
        writeln!(log.lock().unwrap(), "INFO: Received connection from {peer_addr_str}.").unwrap();
        let log_item = LogItem::ConnectLogItem {
            timestamp: SystemTime::now(),
            peer_addr: peer_addr,
        };
        tx.send(log_item).expect("Unable to send on channel.");

        loop {
            //Read exactly one packet from kernel's internal buffer and return it.
            let packet = match handle_packet(&mut connection, &peer_addr_str, Arc::clone(&log)) {
                Ok(p) => Some(p),
                Err(_) => None,
            };

            //Send structured data from packet to main thread.
            if packet.is_some() {
                let log_item = LogItem::PacketLogItem {
                    timestamp: SystemTime::now(),
                    peer_addr: peer_addr,
                    packet: packet.unwrap()
                };

                tx.send(log_item).expect("Unable to send on channel.");
            } else {
                //Send a disconnect notice to packet_log before exiting.
                let log_item = LogItem::DisconnectLogItem {
                    timestamp: SystemTime::now(),
                    peer_addr: peer_addr,
                };
                tx.send(log_item).expect("Unable to send on channel.");
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
        terminal::disable_raw_mode().unwrap();
        execute!(stdout(), terminal::LeaveAlternateScreen).unwrap();
        execute!(stdout(), cursor::Show).unwrap();
    }
}

use std::time::{SystemTime, UNIX_EPOCH};

enum LogItem {
    PacketLogItem {
        timestamp: SystemTime,
        peer_addr: SocketAddr,
        packet: Packet,
    },
    ConnectLogItem {
        timestamp: SystemTime,
        peer_addr: SocketAddr,
    },
    DisconnectLogItem {
        timestamp: SystemTime,
        peer_addr: SocketAddr,
    }
}

impl LogItem {
    fn timestamp(&self) -> SystemTime {
        match self {
            LogItem::PacketLogItem { timestamp, .. } => *timestamp,
            LogItem::ConnectLogItem { timestamp, .. } => *timestamp,
            LogItem::DisconnectLogItem { timestamp, .. } => *timestamp,
        }
    }
}

struct State {
    warn_state: WarnStates,
    warn_state_ascii_art: WarnStateAsciiArt,
    window_should_close: bool,
    packet_log: VecDeque<LogItem>,
    peer_names: HashMap<SocketAddr, String>,

    is_focused_mode: bool,
}

struct RenderState {
    focused_mode_changed: bool,
    warn_state_changed: bool,
    packet_log_changed: bool,

    //For when everything needs to be re-rendered e.g. on resize.
    clear_background: bool,
}

impl RenderState {
    fn new() -> Self {
        return RenderState {
            focused_mode_changed: false,
            warn_state_changed: false,
            packet_log_changed: false,

            clear_background: false,
        };
    }

    fn rerender_all() -> Self {
        return RenderState {
            focused_mode_changed: true,
            warn_state_changed: true,
            packet_log_changed: true,

            clear_background: true,
        };
    }
}

fn print_usage() {
    eprintln!("Usage: ww [Options]");
    eprintln!("Accept networked notifications from client programs.");

    eprintln!("--info-art <Path>: Change the info art with text found at Path. Art must be rectangular to render properly.");
    eprintln!("--warn-art <Path>: Change the warn art with text found at Path. Art must be rectangular to render properly.");
    eprintln!("--alert-art <Path>: Change the alert art with text found at Path. Art must be rectangular to render properly.");

    eprintln!("--help: Show usage and exit.");
}

use std::fs::File;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::{VecDeque, HashMap};
use std::env;

fn main() -> io::Result<()> {
    // env::set_var("RUST_BACKTRACE", "1");
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--help") {
        print_usage();
        std::process::exit(0);
    }

    let listening_port: u16;
    if let Some(i) = args.iter().position(|arg| arg == "-p") {
        if i + 1 < args.len() {
            listening_port = args[i + 1].parse().unwrap_or_else(|_| {
                print_usage();
                std::process::abort();
            });
        }
        else {
            listening_port = 44444;
        }
    }
    else {
        listening_port = 44444;
    }

    let info_art;
    if let Some(i) = args.iter().position(|arg| arg == "--info-art") {
        if i + 1 < args.len() {
            info_art = std::fs::read_to_string(args[i + 1].clone()).unwrap_or_else(|_| {
                print_usage();
                std::process::abort();
            });
        }
        else {
            info_art = WarnStateAsciiArt::default_info_art();
        }
    }
    else {
        info_art = WarnStateAsciiArt::default_info_art();
    }

    let warn_art;
    if let Some(i) = args.iter().position(|arg| arg == "--warn-art") {
        if i + 1 < args.len() {
            warn_art = std::fs::read_to_string(args[i + 1].clone()).unwrap_or_else(|_| {
                print_usage();
                std::process::abort();
            });
        }
        else {
            warn_art = WarnStateAsciiArt::default_warn_art();
        }
    }
    else {
        warn_art = WarnStateAsciiArt::default_warn_art();
    }

    let alert_art;
    if let Some(i) = args.iter().position(|arg| arg == "--alert-art") {
        if i + 1 < args.len() {
            alert_art = std::fs::read_to_string(args[i + 1].clone()).unwrap_or_else(|_| {
                print_usage();
                std::process::abort();
            });
        }
        else {
            alert_art = WarnStateAsciiArt::default_alert_art();
        }
    }
    else {
        alert_art = WarnStateAsciiArt::default_alert_art();
    }

    let mut state = State {
        warn_state: WarnStates::None,
        warn_state_ascii_art: WarnStateAsciiArt::build(info_art, warn_art, alert_art),
        window_should_close: false,
        packet_log: VecDeque::new(),
        peer_names: HashMap::new(),

        is_focused_mode: false,
    };
    let mut render_state = RenderState::rerender_all();
    let mut frame_number: usize = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards.").as_secs() as usize;    //test value 36041;

    let log = Arc::new(Mutex::new(File::create("./warning_window.log")?));

    //Init the window, clean up on drop.
    let _wc = WindowContext::new();

    let (tx, rx) = channel::<LogItem>();
    let mut _log = Arc::clone(&log);

    //The connection_manager thread lives as long as main.
    //It never exits, and continually handles incoming connections.
    let _connection_manager = thread::spawn(move || {
        let listener = TcpListener::bind(format!("localhost:{}", listening_port)).unwrap();

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
        update(&mut state, &mut render_state, &rx, Arc::clone(&log))?;
        //Always render -- after 500 ms or when a key is pressed.
        render(&state, &mut render_state, Arc::clone(&log), frame_number)?;
        frame_number = frame_number.wrapping_add(1);
    }

    return Ok(());
}
