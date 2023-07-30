use std::collections::hash_map::Entry;
use std::{collections::HashMap, io::Write, net::TcpStream, time::Duration};

mod operation;

use crate::config::*;
use operation::*;

use rand::Rng;

#[derive(Debug, PartialEq)]
enum State {
    Leader,
    Follower,
    /// Should be initialized to 1
    Candidate(ClusterSize),
}

pub struct ClusterManager {
    state: State,
    term: TermSize,
    cluster_map: HashMap<ClusterSize, String>,
    node_id: ClusterSize,
}

impl ClusterManager {
    pub fn new(cluster_map: HashMap<ClusterSize, String>, node_id: ClusterSize) -> Self {
        Self {
            state: State::Candidate(1),
            term: 0,
            cluster_map,
            node_id,
        }
    }

    pub fn listen_address(&self) -> &String {
        self.cluster_map.get(&self.node_id).unwrap()
    }

    pub fn handle_timeout(&mut self) {
        match self.state {
            State::Leader => { /* do nothing */ }
            State::Candidate(_) | State::Follower => {
                self.term += 1;
                self.set_state(State::Candidate(1));
            }
        }
    }

    pub fn process_operation(&mut self, raw: Vec<u8>) -> bool {
        let peer_id = raw[0];
        let peer_term = raw[1];
        let operation = match Operation::try_from(raw[2..].to_vec()) {
            Ok(operation) => operation,
            Err(_) => return false,
        };
        log::debug!("{}: {:?}", peer_id, operation);

        if peer_term < self.term {
            if let Entry::Vacant(e) = self.cluster_map.entry(peer_id) {
                if let Operation::Vote(Some(address)) = operation {
                    log::warn!("Receive Vote from new member {}:{}", peer_id, address);
                    e.insert(address);
                }
            }
            return false;
        }

        if peer_term > self.term {
            self.term = peer_term;
            self.set_state(State::Follower);
            if let Operation::Vote(Some(address)) = operation {
                self.send_to_cluster(Some(&address), Operation::Vote(None));
            }
            return true;
        }

        match operation.process(self) {
            OperationResult::Ok => {}
            OperationResult::Response(operation) => {
                let address = &self.cluster_map[&peer_id];
                self.send_to_cluster(Some(address), operation)
            }
            OperationResult::Broadcast(operation) => {
                self.send_to_cluster(None, operation);
            }
            OperationResult::Err => return false,
        }
        true
    }

    pub fn state_action(&self) -> Duration {
        match self.state {
            State::Candidate(1) => {
                self.send_to_cluster(
                    None,
                    Operation::Vote(Some(self.listen_address().to_string())),
                );
            }
            State::Leader => {
                self.send_to_cluster(None, Operation::Heartbeat);
                return Duration::from_millis(HEARTBEAT_INTERVAL);
            }
            _ => {}
        }
        Duration::from_millis(
            rand::thread_rng().gen_range(HEARTBEAT_INTERVAL * 3..HEARTBEAT_INTERVAL * 5),
        )
    }

    fn set_state(&mut self, state: State) {
        log::warn!("Set state: {:?} => {:?}", self.state, state);
        self.state = state;
    }

    fn send_to_cluster(&self, address: Option<&String>, operation: Operation) {
        let mut data: Vec<u8> = Vec::new();
        data.push(self.node_id);
        data.push(self.term);
        data.append(&mut Vec::from(operation));

        if let Some(address) = address {
            match TcpStream::connect(address) {
                Ok(mut stream) => {
                    let _ = stream.write(&data).unwrap();
                    log::trace!("write({:?}) to {}", data, address)
                }
                Err(e) => {
                    log::trace!("connect({}): {}", address, e);
                }
            }
        } else {
            for (peer_id, address) in &self.cluster_map {
                if peer_id == &self.node_id {
                    continue;
                }
                match TcpStream::connect(address) {
                    Ok(mut stream) => {
                        let _ = stream.write(&data).unwrap();
                        log::trace!("write({:?}) to {}", data, address)
                    }
                    Err(e) => {
                        log::trace!("connect({}): {}", address, e);
                    }
                }
            }
        }
    }
}
