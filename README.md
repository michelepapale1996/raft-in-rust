# Raft implementation in Rust

This is a simple implementation of the Raft consensus algorithm in Rust. It is based on the [Raft paper](https://raft.github.io/raft.pdf).

## Running the application
Open multiple terminals and run the following command in each terminal:
```shell
# Node running on port 9092
cargo run -- --cluster-hosts localhost:9093,localhost:9094 --broker-port 9092

# Node running on port 9093
cargo run -- --cluster-hosts localhost:9092,localhost:9094 --broker-port 9093

# Node running on port 9094
cargo run -- --cluster-hosts localhost:9092,localhost:9093 --broker-port 9094
```

## Endpoints

## The project in action

## Limitations (up to now :D)

## Open points & Improvements
- [] Understand logging
- [] Understand the async logic and why we have issues with the scheduler
- [] Use grpc
