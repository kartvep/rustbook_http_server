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
