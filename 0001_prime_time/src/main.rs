/*
    Protohackers problem 1: https://protohackers.com/problem/1
    Prime Time - Accept JSON over TCP, return if passed number is prime number
    Author: Jan Metzger
            https://zazama.de
 */

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::time::Duration;
use rayon::ThreadPoolBuilder;
use serde::{Deserialize, Serialize};
use num_bigint::BigInt;
use serde_json::Number;

#[derive(Serialize, Deserialize)]
struct RequestObject {
    method: String,
    number: Number
}

#[derive(Serialize, Deserialize)]
struct ResponseObjectPrime {
    method: String,
    prime: bool
}

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
                let mut json: String = String::new();
                match buf_reader.read_line(&mut json) {
                    Ok(_) => (),
                    Err(_) => {
                        println!("string malformed: {}", json);
                        send_malformed_answer(&mut stream);
                        break;
                    }
                };

                println!("json: {}", json);

                match serde_json::from_str::<RequestObject>(&json) {
                    Ok(ro) => {
                        if ro.method.ne("isPrime") {
                            println!("method not isPrime");
                            send_malformed_answer(&mut stream);
                            break;
                        }

                        let res = ResponseObjectPrime {
                            method: "isPrime".to_string(),
                            prime: is_prime(BigInt::from_str(&ro.number.to_string()).unwrap_or(BigInt::from(0)))
                        };
                        let res_str = serde_json::to_string(&res).unwrap_or(String::from("{\n")) + "\n";
                        println!("res_str: {}", res_str);
                        stream.write_all(res_str.as_bytes()).unwrap_or_default();
                    },
                    Err(e) => {
                        println!("malformed json: {}", e.to_string());
                        send_malformed_answer(&mut stream);
                        break;
                    }
                };
            }
            println!("connection closed");
        });
    }
}

fn send_malformed_answer(stream: &mut TcpStream) -> bool {
    stream.write_all("{\n".as_bytes()).is_ok()
}

fn is_prime(num: BigInt) -> bool {
    if num.lt(&BigInt::from(2)) {
        return false;
    }

    is_prime::is_prime(&num.to_string())
}

// This code is too slow for large numbers & the test will timeout!
/*fn is_prime(num: i64) -> bool {
    use std::time::Instant;
    let now = Instant::now();

    if num < 2 {
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
        return false;
    }
    for i in 2..num {
        if num % i == 0 {
            let elapsed = now.elapsed();
            println!("Elapsed: {:.2?}", elapsed);
            return false;
        }
    }

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);

    true
}*/