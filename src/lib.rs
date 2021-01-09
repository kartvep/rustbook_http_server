use std::collections::HashMap;
use std::thread;
use std::sync::{mpsc, Arc, Mutex};

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for n in 0..size {
            workers.push(Worker::new(n, Arc::clone(&receiver)));
        }

        ThreadPool{workers, sender}
    }

    pub fn execute<J>(&mut self, job: J )
    where
        J: FnOnce() + Send + 'static,
    {
        let job = Message::NewJob(Box::new(job));
        self.sender.send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {

            match worker.thread.take() {
                Some(thread) => {
                    println!("Shutting down worker {}", worker.id);
                    thread.join().unwrap();
                },
                _ => println!("No thread handler owned by worker {}", worker.id),
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = Some(thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job!", id);
                    job();
                },
                Message::Terminate => {
                    println!("Worker {} got terminate signal", id);
                    break;
                },
            }
        }));

        Worker{ id, thread }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>; 

enum Message {
    NewJob(Job),
    Terminate,
}


#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub http_ver: String,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn from_buffer(buffer: &[u8]) -> Request {
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

        Request{ method, path, http_ver, headers }
    }
}
