/*
    Protohackers problem 0: https://protohackers.com/problem/0
    Smoke Test - Create a TCP echo server
    Author: Jan Metzger
            https://zazama.de
 */

use std::io::{BufReader, Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use rayon::ThreadPoolBuilder;

fn main() {
    // First we read a port from the command line
    let port_str = std::env::args().nth(1).expect("No port provided.");
    // Check if it's a number, otherwise panic
    port_str.parse::<u32>().expect("Port not a number.");
    let listener = TcpListener::bind("0.0.0.0:".to_owned() + &port_str).unwrap();

    // Requirement: "Make [...] that you can handle at least 5 simultaneous clients."
    let pool = ThreadPoolBuilder::new().num_threads(5).build().unwrap();

    for stream in listener.incoming() {
        if stream.is_err() {
            continue;
        }
        let mut stream = stream.unwrap();
        pool.spawn(move || {
            println!("connected");
            let stream_clone = match stream.try_clone() {
                Ok(sc) => sc,
                Err(_) => return
            };
            let timeout = Some(Duration::from_secs(10));
            if stream_clone.set_read_timeout(timeout).is_err() || stream.set_write_timeout(timeout).is_err() {
                return;
            }
            let mut buf_reader = BufReader::new(stream_clone);

            loop {
                let mut buffer = [0u8; 512];
                let read_size = match buf_reader.read(&mut buffer) {
                    Err(_) => break,
                    Ok(b) => b
                };
                println!("read: {}", read_size);

                if read_size == 0 {
                    break;
                }

                match stream.write_all(&buffer[0 .. read_size]) {
                    Ok(_) => {}
                    Err(_) => break
                }
                println!("written");
            }
            println!("connection closed");
        });
    }
}
