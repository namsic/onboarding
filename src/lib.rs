mod message;

use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use rand::Rng;

use message::{Message, MessageType, Response};

const HEARTBEAT_INTERVAL: u64 = 1500;

pub fn start(cluster: HashMap<u8, SocketAddr>, id: u8) {
    let pair = Arc::new((Mutex::new(ClusterManager::new(cluster, id)), Condvar::new()));

    let pair_ = Arc::clone(&pair);
    thread::spawn(move || {
        let (lock, cvar) = &*pair_;
        state_machine(lock, cvar);
    });

    let (lock, _) = &*pair;
    let cluster_manager = lock.lock().unwrap();
    let bind_addr = cluster_manager.cluster_map.get(&id).unwrap();
    let listener = TcpListener::bind(bind_addr).unwrap();
    drop(cluster_manager);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let pair_ = Arc::clone(&pair);
        thread::spawn(move || {
            let (lock, cvar) = &*pair_;
            handle_io(lock, cvar, stream);
        });
    }
}

struct State {
    term: u8,
    role: Role,
}

impl State {
    fn transition(&mut self, term: u8, role: Role) {
        let new_state = Self { term, role };
        log::warn!("{} => {}", self, new_state,);
        *self = new_state;
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "State{{term: {}, role: {}}}", self.term, self.role)
    }
}

enum Role {
    /// State responsible for changes that occur within the cluster
    ///
    /// There can be only one in a cluster.
    Leader(HashMap<u8, TcpStream>),
    Follower,
    /// State before becoming a leader or follower
    ///
    /// Become a leader if node receive more than a quorum vote from other nodes.
    Candidate,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role = match self {
            Self::Leader(_) => "leader",
            Self::Follower => "follower",
            Self::Candidate => "candidate",
        };
        write!(f, "{}", role)
    }
}

struct ClusterManager {
    state: State,
    cluster_map: HashMap<u8, SocketAddr>,
    id: u8,
}

impl ClusterManager {
    fn new(cluster_map: HashMap<u8, SocketAddr>, id: u8) -> Self {
        Self {
            state: State {
                term: 0,
                role: Role::Candidate,
            },
            cluster_map,
            id,
        }
    }

    fn send_heartbeat(&mut self) {
        let Role::Leader(connection_map) = &mut self.state.role else {
            log::error!("send_heartbeat() by {}", self.state.role);
            return;
        };

        for (id, addr) in &self.cluster_map {
            if id == &self.id {
                continue;
            }

            if connection_map.get(id).is_none() {
                match TcpStream::connect(addr) {
                    Ok(stream) => {
                        connection_map.insert(*id, stream);
                    }
                    Err(e) => {
                        log::trace!("send_heartbeat:connect: {}", e)
                    }
                }
            }

            if let Some(stream) = connection_map.get_mut(id) {
                let heartbeat_message = Message {
                    term: self.state.term,
                    body: MessageType::Heartbeat,
                };
                if let Err(e) = heartbeat_message.write_to(stream) {
                    log::error!("send_heartbeat:write_to: {}", e);
                }
            }
        }
    }

    fn request_vote(&mut self) {
        let Role::Candidate = self.state.role else {
            log::error!("request_vote() by {}", self.state.role);
            return;
        };

        let mut vote: u8 = 1;
        let mut connection_map: HashMap<u8, TcpStream> = HashMap::new();

        for (id, addr) in &self.cluster_map {
            if id == &self.id {
                continue;
            }
            let mut stream = match TcpStream::connect(addr) {
                Ok(stream) => stream,
                Err(e) => {
                    log::debug!("request_vote:connect: {}", e);
                    continue;
                }
            };
            let vote_message = Message {
                term: self.state.term,
                body: MessageType::RequestVote,
            };
            if let Err(e) = vote_message.write_to(&mut stream) {
                log::error!("request_vote:write_to: {}", e);
                continue;
            }
            if let Ok(Response::Ack) = Response::read_from(&mut stream) {
                if let Role::Candidate = self.state.role {
                    vote += 1;
                }
            };
            connection_map.insert(*id, stream);
        }

        if vote > self.cluster_map.len() as u8 / 2 {
            self.state
                .transition(self.state.term, Role::Leader(connection_map));
        }
    }

    fn process_message(&mut self, message: Message) -> Response {
        if message.term < self.state.term {
            return Response::Nack;
        }

        if message.term > self.state.term {
            let role = match message.body {
                MessageType::RequestVote => Role::Candidate,
                MessageType::Heartbeat => Role::Follower,
                MessageType::AppendEntry => {
                    log::error!("AppendEntry before Heartbeat");
                    return Response::Nack;
                }
            };
            self.state.transition(message.term, role);
            return Response::Ack;
        }

        match message.body {
            MessageType::RequestVote => {
                return Response::Nack;
            }
            MessageType::Heartbeat => {
                if let Role::Candidate = self.state.role {
                    self.state.transition(self.state.term, Role::Follower);
                }
                return Response::Ack;
            }
            MessageType::AppendEntry => (),
        }

        Response::Ack
    }

    fn timed_out(&mut self) {
        if let Role::Leader(_) = self.state.role {
            self.send_heartbeat();
        } else {
            self.state.transition(self.state.term + 1, Role::Candidate);
            self.request_vote();
        }
    }
}

fn state_machine(lock: &Mutex<ClusterManager>, cvar: &Condvar) {
    loop {
        let guard = lock.lock().unwrap();
        match &guard.state.role {
            Role::Leader(_) => {}
            Role::Follower | Role::Candidate => { /* Just reset the timeout */ }
        }

        let timeout_threshold = if let Role::Leader(_) = &guard.state.role {
            HEARTBEAT_INTERVAL
        } else {
            rand::thread_rng().gen_range(HEARTBEAT_INTERVAL * 3..HEARTBEAT_INTERVAL * 5)
        };

        let (mut guard, timeout_result) = cvar
            .wait_timeout(guard, Duration::from_millis(timeout_threshold))
            .unwrap();

        if timeout_result.timed_out() {
            guard.timed_out();
        }
    }
}

fn handle_io(lock: &Mutex<ClusterManager>, cvar: &Condvar, mut stream: TcpStream) {
    loop {
        let Ok(message) = Message::read_from(&mut stream) else {
            continue;
        };

        let mut guard = lock.lock().unwrap();
        guard.process_message(message).write_to(&mut stream);

        match &guard.state.role {
            Role::Leader(_) => { /* commit */ }
            Role::Follower | Role::Candidate => { /* Response IM_NOT_READER */ }
        }
        cvar.notify_all();
    }
}
