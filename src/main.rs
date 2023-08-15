use std::{collections::HashMap, net::SocketAddr, time::Duration};

fn main() {
    // RUST_LOG=warn cargo run -- 1 (or 2 or 3)
    env_logger::init();

    let mut sample_cluster: HashMap<u8, SocketAddr> = HashMap::new();
    sample_cluster.insert(1, "127.0.0.1:10001".parse().unwrap());
    sample_cluster.insert(2, "127.0.0.1:10002".parse().unwrap());
    sample_cluster.insert(3, "127.0.0.1:10003".parse().unwrap());

    let args: Vec<String> = std::env::args().collect();
    let node_id: u8 = args.get(1).unwrap().parse().unwrap();

    raft_node::start(sample_cluster, node_id);
    loop {
        std::thread::sleep(Duration::from_secs(10));
    }
}
