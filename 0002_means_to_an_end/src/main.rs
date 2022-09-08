use std::cmp::Ordering;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use rayon::ThreadPoolBuilder;
use rotated_array_set::RotatedArraySet;

#[derive(Eq, Copy, Clone, Default, Debug)]
struct Transaction {
    amount: i32,
    timestamp: i32
}

impl PartialEq<Self> for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl PartialOrd<Self> for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
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
            let timeout = Some(Duration::from_secs(10));
            if stream.set_read_timeout(timeout).is_err() || stream.set_write_timeout(timeout).is_err() {
                return;
            }
            let mut transactions: RotatedArraySet<Transaction> = RotatedArraySet::new();

            loop {
                let mut buffer: [u8; 9] = [0; 9];
                match stream.read_exact(&mut buffer) {
                    Ok(s) => s,
                    Err(_) => {
                        println!("Malformed message!");
                        break;
                    }
                };

                match buffer[0] {
                    // I
                    0x49 => {
                        let transaction = evaluate_transaction(&buffer);
                        println!("Inserted {} at {}", transaction.amount, transaction.timestamp);
                        transactions.insert(transaction);
                    },
                    // Q
                    0x51 => {
                        let amount = evaluate_query(&buffer, &transactions);
                        match stream.write_all(&amount.to_be_bytes()) {
                            Ok(_) => {
                                println!("Answer written {}", amount);
                            }
                            Err(_) => {
                                println!("Couldn't send answer");
                            }
                        }
                    },
                    _ => {
                        println!("Invalid method {}", buffer[0]);
                        continue;
                    }
                }
            }
            println!("connection closed");
        });
    }
}

fn evaluate_transaction(buffer: &[u8; 9]) -> Transaction {
    return Transaction {
        amount: i32::from_be_bytes(buffer[5..9].try_into().unwrap()),
        timestamp: i32::from_be_bytes(buffer[1..5].try_into().unwrap())
    }
}

fn evaluate_query(buffer: &[u8; 9], transactions: &RotatedArraySet<Transaction>) -> i32 {
    let min_timestamp = i32::from_be_bytes(buffer[1..5].try_into().unwrap());
    let max_timestamp = i32::from_be_bytes(buffer[5..9].try_into().unwrap());

    println!("Query between {} and {}", min_timestamp, max_timestamp);

    if min_timestamp > max_timestamp {
        return 0;
    }

    let mut current_average = 0f64;
    let mut entries = 1usize;
    for &transaction in transactions.iter() {
        if transaction.timestamp >= min_timestamp && transaction.timestamp <= max_timestamp {
            current_average += (transaction.amount as f64 - current_average) / entries as f64;
            entries += 1;
        }
    }

    if entries == 0 {
        return 0
    }

    return current_average as i32;
}