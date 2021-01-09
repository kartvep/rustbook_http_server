use std::fs;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::AsRawFd;
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
    listener.set_nonblocking(true).expect("Cannot set non-blocking");

    let mut epoll_events = epoll::Events::empty();
    epoll_events.insert(epoll::Events::EPOLLET);
    epoll_events.insert(epoll::Events::EPOLLIN);
    epoll_events.insert(epoll::Events::EPOLLONESHOT);
    println!("{:?}", epoll_events);

    let mut pool = ThreadPool::new(4);
    let queue = epoll::create(true).unwrap();

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

    let mut _events = Vec::new();
    _events.resize_with(10, || {
        epoll::Event::new(epoll::Events::empty(), 1)
    });
    epoll::ctl(
        queue,
        epoll::ControlOptions::EPOLL_CTL_ADD,
        listener.as_raw_fd(),
        epoll::Event::new(epoll_events, 1),
    ).expect("Failed to readd listener to epoll");
    for stream in listener.incoming() {
        if sigint_count.load(Ordering::Relaxed) != 0 {
            break;
        }

        let epoll_events = epoll_events.clone();

        match stream {
            Ok(stream) => {
                println!("Got connection");
                pool.execute(|| {
                    handle_connection(stream);
                });
            },
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Got err! {:?}", e);
                epoll::ctl(
                    queue,
                    epoll::ControlOptions::EPOLL_CTL_MOD,
                    listener.as_raw_fd(),
                    epoll::Event::new(epoll_events, 1),
                ).expect("Failed to readd listener to epoll");
                epoll::wait(queue, -1, &mut _events).unwrap();
            },
            _ => panic!("Bad stream"),
        }
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
