use std::{
    collections::HashMap,
    io::Read,
    net::TcpListener,
    sync::{Arc, Condvar, Mutex},
    thread,
};

mod config;
mod manager;

use crate::manager::*;

pub fn up(cluster_map: HashMap<u8, String>, node_id: u8) {
    let pair = Arc::new((
        Mutex::new(ClusterManager::new(cluster_map, node_id)),
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
    let listener = TcpListener::bind(cluster_manager.listen_address()).unwrap();
    drop(cluster_manager);

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let mut raw = [0; 1024];
        let n = stream.read(&mut raw).unwrap();

        let mut cluster_manager = lock.lock().unwrap();
        if cluster_manager.process_operation(raw[0..n].to_vec()) {
            cvar.notify_all();
        }
    }
}

fn state(pair: Arc<(Mutex<ClusterManager>, Condvar)>) {
    let (lock, cvar) = &*pair;
    loop {
        let cluster_manager = lock.lock().unwrap();
        let dur = cluster_manager.state_action();
        let (mut cluster_manager, timeout_result) =
            cvar.wait_timeout(cluster_manager, dur).unwrap();
        if timeout_result.timed_out() {
            cluster_manager.handle_timeout();
        }
    }
}
