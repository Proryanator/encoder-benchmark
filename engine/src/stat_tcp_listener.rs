use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::num::ParseIntError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use stoppable_thread::StoppableHandle;

use cli::cli_util::error_with_ack;
use ffmpeg::report_files::capture_group;

static LOCALHOST: &str = "localhost";
static PORT: &str = "1234";

pub fn start_listening_to_ffmpeg_stats(
    verbose: bool,
    frame: &'static AtomicUsize,
    previous_frame: &'static AtomicUsize,
) -> StoppableHandle<()> {
    let stat_listener = TcpListener::bind(format!("{}:{}", LOCALHOST, PORT)).unwrap();
    // important so that this thread doesn't just hang here
    stat_listener
        .set_nonblocking(true)
        .expect("Unable to set non-blocking for tcp listener, listener might block...");

    let tcp_reading_thread;

    let listen_start_time = SystemTime::now();
    let allowed_elapsed_time = Duration::from_secs(10);

    loop {
        if listen_start_time.elapsed().unwrap() > allowed_elapsed_time {
            println!("Unable to connect to ffmpeg output for {} seconds, either ffmpeg didn't start correctly or the tcp connection: {}:{} could not be created...", allowed_elapsed_time.as_secs(), LOCALHOST, PORT);
            error_with_ack(true);
        }

        // will try to connect for 10 seconds
        match stat_listener.accept() {
            Ok(client) => {
                if verbose {
                    println!("Connected to ffmpeg's -progress output via TCP...");
                }

                // making received client non-blocking, otherwise it dies pretty quick
                client.0.set_nonblocking(false).unwrap();
                tcp_reading_thread = spawn_tcp_reading_thread(client.0, frame, previous_frame);
                break;
            }
            // probably log this error eventually
            Err(_e) => {
                if verbose {
                    println!("Not able to connect to ffmpeg stat output, will try again...");
                    sleep(Duration::from_secs(1));
                }
            }
        }
    }

    return tcp_reading_thread;
}

fn spawn_tcp_reading_thread(
    stream: TcpStream,
    frame: &'static AtomicUsize,
    previous_frame: &'static AtomicUsize,
) -> StoppableHandle<()> {
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
                frame.store(
                    extract_frame(line.as_str()).unwrap() as usize,
                    Ordering::Relaxed,
                );
            }
        }
    });
}

fn is_frame_line(input: &str) -> bool {
    return input.contains("frame=");
}

pub fn extract_frame(line: &str) -> Result<u64, ParseIntError> {
    return capture_group(line, r"^frame=([0-9]+)").parse::<u64>();
}
