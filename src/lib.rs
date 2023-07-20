use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread,
    time::Duration,
};

mod config;
mod message;

use crate::config::*;
use crate::message::*;

#[derive(Debug)]
enum State {
    Leader,
    Follower,
    Candidate(ClusterSize),
}

pub struct ClusterManager {
    state: State,
    term: TermSize,
    cluster: HashMap<ClusterSize, String>,
    node_id: ClusterSize,
}

//if result.unwrap().1.timed_out() && cluster_manager.state != State::Leader {
//    set_state(cluster_manager, cvar, State::Candidate(1));
//}

impl ClusterManager {
    pub fn start(node_id: ClusterSize, addr: String) {
        let pair = Arc::new((
            Mutex::new(Self {
                state: State::Candidate(1),
                term: 0,
                cluster: HashMap::new(),
                node_id,
            }),
            Condvar::new(),
        ));
        let pair_ = Arc::clone(&pair);
        thread::spawn(move || {
            ClusterManager::listen_thread(pair_, addr);
        });

        let pair_ = Arc::clone(&pair);
        thread::spawn(move || {
            ClusterManager::action_thread(pair_);
        });
    }

    fn listen_thread(pair: Arc<(Mutex<ClusterManager>, Condvar)>, addr: String) {
        let (lock, cvar) = &*pair;
        log::info!("Start to listen: {}", addr);
        let listener = TcpListener::bind(addr).unwrap();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut raw = [0; 1024];
            let n = stream.read(&mut raw).unwrap();
            log::debug!("{:?}", &raw[0..n]);

            // handle received data
        }
    }

    fn action_thread(pair: Arc<(Mutex<ClusterManager>, Condvar)>) {
        let (lock, cvar) = &*pair;
        loop {
            let mut cm = lock.lock().unwrap();
            let wait_ms = cm.action();
            let result = cvar.wait_timeout(cm, Duration::from_millis(wait_ms));
        }
    }

    fn set_state(mut cluster_manager: MutexGuard<ClusterManager>, cvar: &Condvar, state: State) {
        log::warn!("{:?} => {:?}", cluster_manager.state, state);
        cluster_manager.state = state;
        cvar.notify_all();
    }

    fn action(&mut self) -> u64 {
        log::debug!("action: {:?}", self.state);
        match self.state {
            State::Candidate(_) => 1000,
            State::Leader => 100,
            State::Follower => 2000,
        }
    }

    fn send_to_id(&self, node_id: ClusterSize, data: &[u8]) {
        let mut stream = TcpStream::connect(self.cluster.get(&node_id).unwrap()).unwrap();
        stream.write(data).unwrap();
    }
}
