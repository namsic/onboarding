# Namsic cluster node
An implementation of the Raft Consensus Algorithm

```rust
    sample_cluster.insert(1, String::from("127.0.0.1:10001"));
    sample_cluster.insert(2, String::from("127.0.0.1:10002"));
    sample_cluster.insert(3, String::from("127.0.0.1:10003"));
```
```
RUST_LOG=warn cargo run -- 1 (or 2 or 3)
```