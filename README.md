# Raft implementation in Rust

This is a simple implementation of the Raft consensus algorithm in Rust. It is based on the [Raft paper](https://raft.github.io/raft.pdf).

## Running the application
Open multiple terminals and run the following command in each terminal:
```shell
# Node running on port 9092
RUST_LOG=info cargo run -- --cluster-hosts localhost:9093,localhost:9094 --broker-port 9092
```

```shell
# Node running on port 9093
RUST_LOG=info cargo run -- --cluster-hosts localhost:9092,localhost:9094 --broker-port 9093
```

```shell
# Node running on port 9094
RUST_LOG=info cargo run -- --cluster-hosts localhost:9092,localhost:9093 --broker-port 9094
```

## Endpoints
Find the leader and then:
```shell
curl --location --request PUT 'localhost:10093/v1/kv/hello' \
--header 'Content-Type: application/json' \
--data '{
    "value": "world"
}'
```

To retrieve the value associated to hello key:
```shell
curl --location 'localhost:10094/v1/kv/hello'
```

## Features
- [X] Leader Election
- [X] Log Replication
- [ ] Persistence
- [ ] Membership changes
- [ ] Log Compaction

## Open points & Improvements
- [ ] Unit & Functional tests
- [ ] Favor grpc instead of HTTP API calls
- [ ] Decouple the Raft library from the application
- [ ] Remove unwraps and functions that maybe panic