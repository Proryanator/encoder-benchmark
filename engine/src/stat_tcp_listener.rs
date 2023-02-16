use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::num::ParseIntError;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use stoppable_thread::StoppableHandle;
use ffmpeg::report_files::capture_group;

static LOCALHOST: &str = "127.0.0.1";
static PORT: &str = "1234";

pub fn start_listening_to_ffmpeg_stats(verbose: bool, frame: &'static AtomicUsize, previous_frame: &'static AtomicUsize) -> StoppableHandle<()> {
    let stat_listener = TcpListener::bind(format!("{}:{}", LOCALHOST, PORT)).unwrap();

    let tcp_reading_thread;
    match stat_listener.accept() {
        Ok(client) => {
            if verbose {
                println!("Connected to ffmpeg's -progress output via TCP...");
            }

            tcp_reading_thread = spawn_tcp_reading_thread(client.0, frame, previous_frame);
        }
        // probably log this error eventually
        Err(_e) => {
            println!("Not able to connect to client for reading stats, cannot proceed");
            process::exit(1);
        }
    }

    // eventually we'll want to add code where we kill the listener here

    return tcp_reading_thread;
}

fn spawn_tcp_reading_thread(stream: TcpStream, frame: &'static AtomicUsize, previous_frame: &'static AtomicUsize) -> StoppableHandle<()> {
    return stoppable_thread::spawn(move |stopped| {
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        let mut peek = [0u8];
        while stream.peek(&mut peek).is_ok() {
            if stopped.get() {
                break;
            }

            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            if is_frame_line(line.as_str()) {
                previous_frame.store(frame.load(Ordering::Relaxed), Ordering::Relaxed);
                frame.store(extract_frame(line.as_str()).unwrap() as usize, Ordering::Relaxed);
            }
        }
    });
}

fn is_frame_line(input: &str) -> bool {
    return input.contains("frame=");
}

pub fn extract_frame(line: &str) -> Result<u64, ParseIntError> {
    return capture_group(line, r"^frame=([0-9]+)")
        .parse::<u64>();
}