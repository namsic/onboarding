use std::{collections::HashMap, time::Duration};

fn main() {
    // RUST_LOG=warn cargo run -- 1 (or 2 or 3)
    env_logger::init();

    let mut sample_cluster: HashMap<u8, String> = HashMap::new();
    sample_cluster.insert(1, String::from("127.0.0.1:10001"));
    sample_cluster.insert(2, String::from("127.0.0.1:10002"));
    sample_cluster.insert(3, String::from("127.0.0.1:10003"));

    let args: Vec<String> = std::env::args().collect();
    let node_id: u8 = args.get(1).unwrap().parse().unwrap();

    raft_node::up(sample_cluster, node_id);
    loop {
        std::thread::sleep(Duration::from_secs(10));
    }
}
