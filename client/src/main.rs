use api::Session;
use adhocrays::*;

fn button(dc: &mut DrawingContext, x: i32, y: i32, w: i32, h: i32, text: &str, bg_color: Color) -> bool {
    let mouse_pos = get_mouse_position();
    let mouse_x = mouse_pos.x as i32;
    let mouse_y = mouse_pos.y as i32;
    let mut is_pressed = false;

    if mouse_x >= x && mouse_x <= x + w
        && mouse_y >= y && mouse_y <= y + h {

        if is_mouse_button_pressed(MouseButton::Left) {
            dc.draw_rectangle(x, y, w, h, Color { r: 200, g: 200, b: 200, a: 255 });
            is_pressed = true;
        }
        else {
            dc.draw_rectangle(x, y, w, h, Color { r: 100, g: 100, b: 100, a: 255 });
        }

        dc.draw_rectangle(x + 2, y + 2, w - 4, h - 4, bg_color);
    }
    else {
        dc.draw_rectangle(x, y, w, h, bg_color);
    }

    let font_size = 20;
    let ascii_size = measure_text_ex(get_default_font(), text, font_size as f32, 1.5);
    dc.draw_text(text, x + w/2 - (ascii_size.x / 2.0) as i32, y + h/2 - (ascii_size.y / 2.0) as i32, 20, colors::WHITE);

    return is_pressed;
}

use std::time::{Duration, Instant};
use std::thread;

fn sleep_until(time: Instant) {
    if time > Instant::now() {
        thread::sleep(time - Instant::now());
    }
}

fn main() {
    let mut session = match Session::connect("localhost:44444") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            return;
        },
    };

    let wc = init_window_context(800, 450, "warn_client");
    let mut msg = String::new();
    let mut err_msg = String::new();

    let max_fps = 30.0;
    let frame_time = Duration::from_secs_f32(1.0/max_fps);
    let mut next_frame = Instant::now();

    while !wc.window_should_close() {
        //Sleep until next frame.
        sleep_until(next_frame);
        next_frame += frame_time;

        let mut dc = wc.init_drawing_context();
        dc.clear_background(Color { r: 25, g: 75, b: 75, a: 255 });

        //Get input into msg.
        let char_pressed = get_char_pressed();
        if char_pressed.is_some() {
            err_msg = "".to_string();
            msg.push(char_pressed.unwrap());
        }

        if is_key_pressed(Key::BACKSPACE) || is_key_pressed_repeat(Key::BACKSPACE) {
            err_msg = "".to_string();
            msg.pop();
        }

        let middle_height = get_screen_height() / 2;
        let middle_width = get_screen_width() / 2;

        //Draw the title.
        let font_size = 25;
        let txt = "Warn Client";
        let ascii_size = measure_text_ex(get_default_font(), txt, font_size as f32, 1.5);
        let x = middle_width - (ascii_size.x / 2.0) as i32;
        let y = middle_height - (ascii_size.y / 2.0) as i32;
        dc.draw_text(txt, x, y - 170, font_size, Color { r: 244, g: 131, b: 37, a: 255 });

        //Draw the message that will be sent upon INFO/WARN/ALERT, etc.
        let font_size = 20;
        let ascii_size = measure_text_ex(get_default_font(), &msg, font_size as f32, 1.5);
        let x = middle_width - (ascii_size.x / 2.0) as i32;
        let y = middle_height - (ascii_size.y / 2.0) as i32;
        dc.draw_text(&msg, x, y - 70, font_size, colors::WHITE);

        let txt = "Sending:";
        let ascii_size = measure_text_ex(get_default_font(), txt, font_size as f32, 1.5);
        let x = middle_width - (ascii_size.x / 2.0) as i32;
        let y = middle_height - (ascii_size.y / 2.0) as i32;
        dc.draw_text(txt, x, y - 90, font_size, colors::WHITE);

        //Draw the error message.
        let color;
        if err_msg.starts_with("ERR:") {
            color = colors::RED;
        }
        else {
            color = colors::GREEN;
        }
        let ascii_size = measure_text_ex(get_default_font(), &err_msg, font_size as f32, 1.5);
        let x = middle_width - (ascii_size.x / 2.0) as i32;
        let y = middle_height - (ascii_size.y / 2.0) as i32;
        dc.draw_text(&err_msg, x, y - 120, font_size, color);

        //Now draw the buttons:

        let w = 150;
        let h = 50;
        let offset = 0;
        let x = middle_width - (w / 2);
        let y = middle_height - (h / 2) + offset;
        if button(&mut dc, x, y, w, h, "INFO", Color { r: 24, g: 24, b: 24, a: 255 }) {
            if msg.len() == 0 {
                err_msg = "ERR: INFO messages must be non-zero.".to_string();
            }
            else {
                match session.send_info(&msg) {
                    Ok(_) => err_msg = "Sent!".to_string(),
                    Err(e) => err_msg = format!("ERR: {}", e),
                }
            }
        }

        let w = 150;
        let h = 50;
        let offset = 70;
        let x = middle_width - (w / 2);
        let y = middle_height - (h / 2) + offset;
        if button(&mut dc, x, y, w, h, "WARN", Color { r: 244, g: 131, b: 37, a: 255 }) {
            match session.send_warn(&msg) {
                Ok(_) => err_msg = "Sent!".to_string(),
                Err(e) => err_msg = format!("ERR: {}", e),
            }
        }

        let w = 150;
        let h = 50;
        let offset = 140;
        let x = middle_width - (w / 2);
        let y = middle_height - (h / 2) + offset;
        if button(&mut dc, x, y, w, h, "ALERT", Color { r: 179, g: 0, b: 0, a: 255 }) {
            match session.send_alert(&msg) {
                Ok(_) => err_msg = "Sent!".to_string(),
                Err(e) => err_msg = format!("ERR: {}", e),
            }
        }
    }
}
