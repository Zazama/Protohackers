use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::thread;
use std::thread::current;
use std::time::Duration;
use rayon::ThreadPoolBuilder;

fn main() {
    // First we read a port from the command line
    let port_str = std::env::args().nth(1).expect("No port provided.");
    // Check if it's a number, otherwise panic
    port_str.parse::<u32>().expect("Port not a number.");
    let listener = TcpListener::bind("0.0.0.0:".to_owned() + &port_str).unwrap();

    // Requirement: "Make [...] that you can handle at least 5 simultaneous clients."
    let pool = ThreadPoolBuilder::new().num_threads(10).build().unwrap();

    let connected_clients: Arc<RwLock<HashMap<String, Vec<String>>>> = Arc::new(RwLock::new(HashMap::new()));

    for stream in listener.incoming() {
        if stream.is_err() {
            continue;
        }
        let mut stream = stream.unwrap();
        let connected_clients = connected_clients.clone();
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
            println!("Sending name request");
            match stream.write_all("Welcome to budgetchat! What shall I call you?\n".as_bytes()) {
                Ok(_) => {}
                Err(_) => { return; }
            }
            println!("Waiting for name");
            let mut username = String::new();
            match buf_reader.read_line(&mut username) {
                Ok(_) => {}
                Err(_) => { return; }
            }
            username = username.replace("\r", "").replace("\n", "");

            println!("Test username: {}", &username);

            {
                if !is_valid_username(&username) || connected_clients.read().unwrap().contains_key(&username) {
                    println!("Invalid or duplicate username");
                    return;
                }
            }

            println!("Valid user: {}", &username);

            {
                let mut present_clients: Vec<String> = Vec::new();
                let mut hm = connected_clients.write().unwrap();
                let keys = hm.keys().cloned().collect::<Vec<_>>();
                for key in keys {
                    present_clients.push(key.clone());
                    hm.get_mut(&key).unwrap().push(format!("* {} has entered the room", &username));
                }

                hm.insert(username.clone(), vec![format!("* The room contains: {}", present_clients.join(", "))]);
            }

            let username_arc = Arc::new(username.clone());
            let username_write = username_arc.clone();
            let cc_write = connected_clients.clone();
            let write_handle = thread::spawn(move || {
                loop {
                    let mut read_lock = cc_write.read().unwrap();
                    let current_vec = match read_lock.get(&*username_write) {
                        None => {
                            println!("User doesn't exist in hm {}", &username_write);
                            break;
                        }
                        Some(v) => v
                    };
                    let vec_len = current_vec.len();
                    std::mem::drop(read_lock);
                    if vec_len == 0 {
                        thread::sleep(Duration::from_millis(100));
                    } else {
                        println!("Send message to {}", &username_write);
                        let mut hm = cc_write.write().unwrap();
                        let message = hm.get_mut(&*username_write).unwrap().remove(0);
                        println!("{}", &message);
                        match stream.write_all((message + "\n").as_bytes()) {
                            Ok(_) => {}
                            Err(_) => { break; }
                        }
                    }
                }

                stream.shutdown(Shutdown::Both);
                println!("Shutdown write");
            });

            let username_read = username_arc.clone();
            let cc_read = connected_clients.clone();
            let read_handle = thread::spawn(move || {
                loop {
                    let mut message = String::new();
                    match buf_reader.read_line(&mut message) {
                        Ok(_) => {}
                        Err(_) => { break; }
                    }
                    message = message.replace("\n", "").replace("\r", "");

                    if !is_valid_message(&message) {
                        println!("Invalid message '{}'", &message);
                        break;
                    }

                    let mut hm = cc_read.write().unwrap();
                    let keys = hm.keys().cloned().collect::<Vec<_>>();
                    for key in keys {
                        if &key != &*username_read {
                            println!("Push message '{}' to user {}", &message, &username_read);
                            hm.get_mut(&key).unwrap().push(format!("[{}] {}", &username_read, &message));
                        }
                    }
                }

                let mut hm = cc_read.write().unwrap();
                hm.remove(&*username).unwrap_or_default();
                let keys = hm.keys().cloned().collect::<Vec<_>>();
                for key in keys {
                    hm.get_mut(&key).unwrap().push(format!("* {} has left the room", &username_read));
                }

                buf_reader.get_mut().shutdown(Shutdown::Both);
                println!("Shutdown read");
            });

            write_handle.join();
            read_handle.join();

            println!("connection closed");
        });
    }
}

fn is_valid_username(username: &str) -> bool {
    if username.len() < 1 || username.len() > 50 {
        return false;
    }

    return username.is_ascii() && username.chars().all(char::is_alphanumeric)
}

fn is_valid_message(message: &str) -> bool {
    return message.is_ascii() && message.len() >= 1;
}