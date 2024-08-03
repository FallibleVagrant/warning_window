use api::Session;

fn main() {
    let mut session = match Session::connect("localhost:44444") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not connect: {}", e);
            return;
        },
    };

    session.send_warn("incoming").unwrap();
    session.send_info("yes").unwrap();
    session.send_alert("no").unwrap();
    session.change_name("hi").unwrap();
    loop {}
}
