//use std::io::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("[::]:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Got connection");
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    #[derive(Debug)]
    struct Request {
        method: String,
        path: String,
        http_ver: String,
        headers: HashMap<String, String>,
    }

    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let request_str = String::from_utf8_lossy(&buffer[..]);
    let mut request_lines = request_str
        .split("\r\n")
        .enumerate()
        .map(|(n, l)| {
            if n == 0 {
                l.split(" ")
            } else {
                l.split(": ")
            }
        });

    let mut req = request_lines.next().unwrap().map(|w| {
        w.to_string()
    });
    let method = req.next().unwrap();
    let path = req.next().unwrap();
    let http_ver = req.next().unwrap();

    let mut headers = HashMap::new();
    for line in request_lines {
        let line = line.collect::<Vec<&str>>();
        if line.len() < 2 {
            break;
        } else {
            headers.insert(
                line[0].to_string(), 
                line[1].to_string(),
            );
        }
    }

    let request = Request{ method, path, http_ver, headers };
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
                _ => format!("HTTP/1.1 404 Not found\r\n\r\nNot found"),
            }
        },
        _ => format!("HTTP/1.1 501 Not Implemented\r\n\r\n"),
    };
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
