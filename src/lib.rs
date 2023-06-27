use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

mod config;
mod operation;

use rand::Rng;

use crate::config::*;
use crate::operation::*;

#[derive(Debug, PartialEq)]
enum State {
    Leader,
    Follower,
    /// Should be initialized to 1
    Candidate(ClusterSize),
}

struct ClusterManager {
    state: State,
    term: TermSize,
    cluster: HashMap<ClusterSize, String>,
    node_id: ClusterSize,
}

pub fn up(cluster_map: HashMap<u8, String>, node_id: u8) {
    let pair = Arc::new((
        Mutex::new(ClusterManager {
            state: State::Candidate(1),
            term: 0,
            cluster: cluster_map,
            node_id,
        }),
        Condvar::new(),
    ));

    let pair_ = Arc::clone(&pair);
    thread::spawn(move || {
        service(pair_);
    });

    let pair_ = Arc::clone(&pair);
    thread::spawn(move || {
        state(pair_);
    });
}

fn service(pair: Arc<(Mutex<ClusterManager>, Condvar)>) {
    let (lock, cvar) = &*pair;
    let cluster_manager = lock.lock().unwrap();
    let address = cluster_manager
        .cluster
        .get(&cluster_manager.node_id)
        .unwrap();
    let listener = TcpListener::bind(address).unwrap();
    drop(cluster_manager);

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut raw = [0; 1024];
        let n = stream.read(&mut raw).unwrap();
        if n < 3 {
            continue;
        }
        let peer_id = raw[0];
        let msg_term = raw[1];
        let operation = match Operation::try_from(raw[2..n].to_vec()) {
            Ok(operation) => operation,
            Err(_) => continue,
        };

        let mut cluster_manager = lock.lock().unwrap();
        match cluster_manager.process_operation(peer_id, msg_term, operation) {
            OperationResult::Ignore => continue,
            OperationResult::Ok => (),
        }
        cvar.notify_all()
    }
}

fn state(pair: Arc<(Mutex<ClusterManager>, Condvar)>) {
    let (lock, cvar) = &*pair;
    loop {
        let cluster_manager = lock.lock().unwrap();
        let timeout_ms = cluster_manager.state_action();

        let (mut cluster_manager, timeout_result) = cvar
            .wait_timeout(cluster_manager, Duration::from_millis(timeout_ms))
            .unwrap();
        if let State::Leader = cluster_manager.state {
            continue;
        }
        if timeout_result.timed_out() {
            cluster_manager.term += 1;
            cluster_manager.set_state(State::Candidate(1))
        }
    }
}

impl ClusterManager {
    fn process_operation(
        &mut self,
        peer_id: ClusterSize,
        peer_term: TermSize,
        operation: Operation,
    ) -> OperationResult {
        log::trace!(
            "process_operation(id:{}, term:{}, {:?})",
            peer_id,
            peer_term,
            operation
        );
        if self.term > peer_term {
            return OperationResult::Ignore;
        }
        if self.term < peer_term {
            self.term = peer_term;
            if let Operation::Vote(true) = operation {
                self.send_message_to_id(peer_id, Operation::Vote(false));
            }
            self.set_state(State::Follower);
            return OperationResult::Ok;
        }

        match operation {
            Operation::Heartbeat => OperationResult::Ok,
            Operation::Vote(false) => {
                if let State::Candidate(n) = self.state {
                    if n < self.cluster.len() as u8 / 2 {
                        self.set_state(State::Candidate(n + 1))
                    } else {
                        self.set_state(State::Leader)
                    }
                    OperationResult::Ok
                } else {
                    OperationResult::Ignore
                }
            }
            Operation::Vote(true) => OperationResult::Ignore,
        }
    }

    fn set_state(&mut self, state: State) {
        log::warn!("Set state: {:?} => {:?}", self.state, state);
        self.state = state;
    }

    fn state_action(&self) -> u64 {
        match self.state {
            State::Candidate(1) => {
                self.broadcast_message(Operation::Vote(true));
            }
            State::Leader => {
                self.broadcast_message(Operation::Heartbeat);
                return HEARTBEAT_INTERVAL;
            }
            _ => {}
        }
        rand::thread_rng().gen_range(HEARTBEAT_INTERVAL * 3..HEARTBEAT_INTERVAL * 5)
    }

    fn send_message_to_id(&self, peer_id: ClusterSize, operation: Operation) {
        log::trace!("send_message_to_id({}, {:?})", peer_id, operation);
        let mut data: Vec<u8> = Vec::new();
        data.push(self.node_id);
        data.push(self.term);
        data.append(&mut Vec::from(operation));

        let address = match self.cluster.get(&peer_id) {
            Some(address) => address,
            None => return,
        };
        match TcpStream::connect(address) {
            Ok(mut stream) => {
                let n = stream.write(&data).unwrap();
                if n < data.len() {
                    log::error!("write {}/{}", n, data.len());
                }
            }
            Err(e) => {
                log::trace!("{:?}: {}", address, e);
            }
        }
    }

    fn broadcast_message(&self, operation: Operation) {
        log::trace!("broadcast_message({:?})", operation);
        let mut data: Vec<u8> = Vec::new();
        data.push(self.node_id);
        data.push(self.term);
        data.append(&mut Vec::from(operation));

        for (peer_id, address) in &self.cluster {
            if peer_id == &self.node_id {
                continue;
            }
            match TcpStream::connect(address) {
                Ok(mut stream) => {
                    let n = stream.write(&data).unwrap();
                    if n < data.len() {
                        log::error!("write {}/{}", n, data.len());
                    }
                }
                Err(e) => {
                    log::trace!("{:?}: {}", address, e)
                }
            }
        }
    }
}
