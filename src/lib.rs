use std::thread;
use std::sync::{mpsc, Arc, Mutex};

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
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

    pub fn execute<T>(&mut self, job: T )
    where
        T: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id: usize,
    join_handle: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();

                println!("Worker {} got a job!", id);
                
                job();
            }
        });

        Worker{ id, join_handle: thread }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>; 
