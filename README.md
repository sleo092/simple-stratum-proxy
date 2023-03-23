## About

This is a simple proxy that receives stratum V1 messages from miners and forwards them to the mining pool and vice versa.

All messages are deserialized to Rust structs and the server prints this data to the terminal.

These data can eventually be stored in a database.

## Clone and run the projet

```
git clone https://github.com/sleo092/simple-stratum-proxy.git
cd simple-stratum-proxy
cargo run
```

## Run CPU Miner

. Download [CPU Miner](https://github.com/pooler/cpuminer) from https://sourceforge.net/projects/cpuminer/files/

. Run `./minerd -a sha256d -o stratum+tcp://localhost:34255 -q -D -P -u <legacy_btc_address> -p x` 

The proxy server will display the messages received from CPU Miner and from mining pool.