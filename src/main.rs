use std::time::Duration;

use raft_node;

fn main() {
    env_logger::init();
    for i in 1..10 {
        println!("{}: {}", i, calc_quorum(i));
    }
    raft_node::ClusterManager::start(1, String::from("0.0.0.0:41234"));
    loop {
        println!(".");
        std::thread::sleep(Duration::from_secs(10));
    }
}

fn calc_quorum(a: u8) -> u8 {
    return a / 2 + 1;
}
