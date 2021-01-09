use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering}; 
use std::thread;
use std::time;

use signal_hook::{iterator::Signals, consts::SIGINT};

use server::ThreadPool;

fn main() {
    let mut signals = Signals::new(&[SIGINT]).unwrap();
    let sigint_count = Arc::new(AtomicUsize::new(0));
    let sigint_count_thread = Arc::clone(&sigint_count);
    let listener = TcpListener::bind("[::]:7878").unwrap();
    let mut pool = ThreadPool::new(4);

    thread::spawn(move || {
        for s in signals.forever() {
            match s {
                SIGINT => {
                    println!("Got SIGINT!");
                    if sigint_count_thread.load(Ordering::Relaxed) == 0 {
                        println!("Stop handling new connections! Ctrl+C again to exit immediately");
                        sigint_count_thread.store(1, Ordering::Relaxed);
                    } else {
                        std::process::exit(1);
                    }
                },
                _ => unreachable!(),
            }
        }
        signals.handle().close();
    });

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        if sigint_count.load(Ordering::Relaxed) != 0 {
            break
        }

        println!("Got connection");
        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let request = server::Request::from_buffer(&buffer);
    println!("{:?}", request);

    let contents = fs::read_to_string("hello.html").unwrap();

    let response = match &request.method[..] {
        "GET" => {
            match &request.path[..] {
                "/" | "/hello.html" | "/index.html" => format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                    contents.len(),
                    contents,
                ),
                "/sleep.html" => {
                    thread::sleep(time::Duration::from_secs(5));
                    format!("HTTP/1.1 200 OK\r\n\r\nToo slow")
                },
                _ => format!("HTTP/1.1 404 Not found\r\n\r\nNot found"),
            }
        },
        _ => format!("HTTP/1.1 501 Not Implemented\r\n\r\n"),
    };
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
