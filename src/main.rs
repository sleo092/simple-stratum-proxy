use bytes::BytesMut;
use hex;
use serde_json::{from_slice, Value};
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;

const MINING_POOL: &str = "solo.ckpool.org:3333";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:34255".to_string());
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }

    Ok(())
}

async fn handle_connection(client_stream: TcpStream) {
    if let Err(e) = proxy(client_stream).await {
        eprintln!("Error while proxying: {}", e);
    }
}

async fn handle_mining_subscribe(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let user_agent: &str = params.get(0).and_then(Value::as_str).unwrap_or("");
        let session_id: &str = params.get(1).and_then(Value::as_str).unwrap_or("");

        println!("User Agent: {}", user_agent);
        println!("Session ID: {}", session_id);
    }
}

async fn handle_mining_set_difficulty(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let difficulty = params.get(0).and_then(Value::as_f64).unwrap_or(0.0);

        println!("Difficulty: {}", difficulty);
    }
}

#[derive(Debug)]
struct MiningNotify {
    job_id: String,
    prevhash: String,
    coinb1: Vec<u8>,
    coinb2: Vec<u8>,
    merkle_branch: Vec<String>,
    version: u32,
    nbits: u32,
    ntime: u32,
    clean_jobs: bool,
}

impl Display for MiningNotify {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "MiningNotify {{job_id: {},prevhash: {},coinb1: {},coinb2: {},merkle_branch: {:?},version: {},nbits: {},ntime: {},clean_jobs: {}}}",
            self.job_id,
            self.prevhash,
            hex::encode(&self.coinb1),
            hex::encode(&self.coinb2),
            self.merkle_branch,
            self.version,
            self.nbits,
            self.ntime,
            self.clean_jobs,
        )
    }
}

async fn handle_mining_notify(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let notify_data = MiningNotify {
            job_id: params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            prevhash: params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            coinb1: params
                .get(2)
                .and_then(Value::as_str)
                .map(hex::decode)
                .unwrap_or_else(|| Ok(vec![]))
                .unwrap_or_else(|_| vec![]),
            coinb2: params
                .get(3)
                .and_then(Value::as_str)
                .map(hex::decode)
                .unwrap_or_else(|| Ok(vec![]))
                .unwrap_or_else(|_| vec![]),
            merkle_branch: params
                .get(4)
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_else(|| vec![]),
            version: params.get(5).and_then(Value::as_u64).unwrap_or(0) as u32,
            nbits: params.get(6).and_then(Value::as_u64).unwrap_or(0) as u32,
            ntime: params.get(7).and_then(Value::as_u64).unwrap_or(0) as u32,
            clean_jobs: params.get(8).and_then(Value::as_bool).unwrap_or(false),
        };

        println!("Mining Notify: {}", notify_data);
    }
}

#[derive(Debug)]
struct MiningSetExtranonce {
    extranonce1: String,
    extranonce2_size: u64,
}

impl Display for MiningSetExtranonce {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "MiningSetExtranonce {{extranonce1: {}, extranonce2_size: {}\n}}",
            self.extranonce1, self.extranonce2_size,
        )
    }
}

async fn handle_mining_set_extranonce(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let extranonce_data = MiningSetExtranonce {
            extranonce1: params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            extranonce2_size: params.get(1).and_then(Value::as_u64).unwrap_or(0),
        };

        println!("Mining Set Extranonce: {}", extranonce_data);
    }
}

#[derive(Debug)]
struct MiningAuthorize {
    worker_name: String,
    worker_password: String,
}

impl Display for MiningAuthorize {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "MiningAuthorize {{worker_name: {}, worker_password: {}}}",
            self.worker_name, self.worker_password,
        )
    }
}

async fn handle_mining_authorize(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let authorize_data = MiningAuthorize {
            worker_name: params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            worker_password: params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        };

        println!("Mining Authorize: {}", authorize_data);
    }
}

#[derive(Debug)]
struct MiningSubmit {
    worker_name: String,
    job_id: String,
    extranonce2: Vec<u8>,
    ntime: u32,
    nonce: u32,
}

impl Display for MiningSubmit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "MiningSubmit {{worker_name: {}, job_id: {}, extranonce2: {}, ntime: {:08x}, nonce: {:08x}}}",
            self.worker_name,
            self.job_id,
            hex::encode(&self.extranonce2),
            self.ntime,
            self.nonce,
        )
    }
}

async fn handle_mining_submit(json_msg: &Value) {
    if let Value::Array(ref params) = json_msg["params"] {
        let submit_data = MiningSubmit {
            worker_name: params
                .get(0)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            job_id: params
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            extranonce2: params
                .get(2)
                .and_then(Value::as_str)
                .map(|s| hex::decode(s).unwrap_or_default())
                .unwrap_or_default(),
            ntime: params
                .get(3)
                .and_then(Value::as_str)
                .map(|s| u32::from_str_radix(s, 16).unwrap_or(0))
                .unwrap_or(0),
            nonce: params
                .get(4)
                .and_then(Value::as_str)
                .map(|s| u32::from_str_radix(s, 16).unwrap_or(0))
                .unwrap_or(0),
        };
        println!("Mining Submit: {:?}", submit_data);
    }
}

#[derive(Debug)]
struct MiningExtraNonceSubscribe {
    request_id: u64,
}

impl Display for MiningExtraNonceSubscribe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "MiningExtraNonceSubscribe {{\n  request_id: {}\n}}",
            self.request_id,
        )
    }
}

async fn handle_extranonce_subscribe(json_msg: &Value) {
    if let Some(id) = json_msg["id"].as_u64() {
        let extra_nonce_subscribe_data = MiningExtraNonceSubscribe {
            request_id: id,
        };

        println!("Mining ExtraNonce Subscribe: {}", extra_nonce_subscribe_data);
    }
}

async fn proxy(client_stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let pool_stream = TcpStream::connect(MINING_POOL).await?;
    let (mut client_reader, mut client_writer) = io::split(client_stream);
    let (mut pool_reader, mut pool_writer) = io::split(pool_stream);

    let client_to_pool = async {
        let mut buffer = BytesMut::with_capacity(4096);
        loop {
            buffer.clear();
            let n = client_reader.read_buf(&mut buffer).await?;
            if n == 0 {
                break;
            }
            println!("Received from CPUMiner: {:?}", buffer);

            // Deserialize and handle messages
            if let Ok(json_msg) = from_slice::<Value>(&buffer) {
                match json_msg["method"].as_str() {
                    Some("mining.subscribe") => handle_mining_subscribe(&json_msg).await,
                    Some("mining.authorize") => handle_mining_authorize(&json_msg).await,
                    Some("mining.submit") => handle_mining_submit(&json_msg).await,
                    Some("mining.extranonce.subscribe") => handle_extranonce_subscribe(&json_msg).await,
                    _ => {}
                }
            }

            pool_writer.write_all(&buffer).await?;
        }
        pool_writer.shutdown().await?;
        Result::<_, Box<dyn Error>>::Ok(())
    };

    let pool_to_client = async {
        let mut buffer = BytesMut::with_capacity(4096);
        loop {
            buffer.clear();
            let n = pool_reader.read_buf(&mut buffer).await?;
            if n == 0 {
                break;
            }
            println!("Received from mining pool: {:?}", buffer);

            // Deserialize and handle messages
            if let Ok(json_msg) = from_slice::<Value>(&buffer) {
                match json_msg["method"].as_str() {
                    Some("mining.set_difficulty") => handle_mining_set_difficulty(&json_msg).await,
                    Some("mining.notify") => handle_mining_notify(&json_msg).await,
                    Some("mining.set_extranonce") => handle_mining_set_extranonce(&json_msg).await,
                    _ => {}
                }
            }

            client_writer.write_all(&buffer).await?;
        }
        client_writer.shutdown().await?;
        Result::<_, Box<dyn Error>>::Ok(())
    };

    select! {
        res1 = client_to_pool => res1,
        res2 = pool_to_client => res2,
    }
}
