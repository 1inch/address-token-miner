use tiny_keccak::{Keccak, Hasher};
use std::thread;
use std::time::Instant;
use structopt::StructOpt;
use std::time::{SystemTime, UNIX_EPOCH};

extern crate hex;

#[derive(StructOpt)]
struct Args {
    deployer: String,
    wallet: String,
    prehex: String,
    #[structopt(default_value = "1")]
    threads: usize,
    #[structopt(default_value = "21c35dbe1b344a2488cf3321d6ce542f8e9f305544ff09e4993a62319a497c1f")]
    proxy_bytecode_hash: String,
}

fn main() {
    let args = Args::from_args();

    let deployer = hex::decode(args.deployer.strip_prefix("0x").unwrap_or(&args.deployer)).unwrap();
    let proxy_bytecode_hash = hex::decode(args.proxy_bytecode_hash.strip_prefix("0x").unwrap_or(&args.proxy_bytecode_hash)).unwrap();

    let mut data = [0u8; 32];
    let wallet = args.wallet.strip_prefix("0x").unwrap_or(&args.wallet);
    let mut address_hasher = Keccak::v256();
    // println!("Wallet: {:?}", &hex::decode(wallet).unwrap().as_slice());
    address_hasher.update(&hex::decode(wallet).unwrap().as_slice());
    address_hasher.finalize(&mut data);
    data[0..16].copy_from_slice(&[0u8; 16]);

    let prehex = args.prehex.strip_prefix("0x").unwrap_or(&args.prehex).to_string();
    let prefix = hex::decode(&prehex).unwrap();
    // println!("Prefix: {}", prehex);
        
    let args_threads = args.threads;
    let mut handles = vec![];
    for ti in 0..args.threads as u8 {
        let prehex_clone = prehex.clone();
        let prefix_clone = prefix.clone();
        let deployer_clone = deployer.clone();
        let proxy_bytecode_hash_clone = proxy_bytecode_hash.clone();
        handles.push(Some(thread::spawn(move || {
            let random = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

            let mut buffer = data.clone();

            let mut index = 0;
            let mut reported_index = 0;
            let mut last = Instant::now();
            let first = last;

            buffer[0] = ti as u8;
            buffer[1..8].copy_from_slice(&random.to_le_bytes()[0..7]);
            for i2 in 0..0xffffffffffffffffu64 {
                buffer[8..16].copy_from_slice(&i2.to_le_bytes());

                index += 1;
                let ms = last.elapsed().as_millis() as u64;
                if ms > 3000 {
                    println!(
                        "Thread #{:x}: iteration {}M ({} MH/s)\r",
                        ti,
                        (index / 1000) as f64 / 1000.0,
                        ((index - reported_index) / (1 + ms)) as f64 / 1000.0
                    );
                    last = Instant::now();
                    reported_index = index
                }

                let mut res = [0u8; 32];

                // proxy_addr = '0x' + Web3.solidity_keccak(
                //     ['bytes1', 'address', 'bytes32', 'bytes32'],
                //     ['0xff', '0x1Add4e558Ce81fbdFD097550894CBdF37D448a9E', salt, PROXY_BYTECODE_HASH]
                // ).hex()[-40:]
                let mut hasher = Keccak::v256();
                hasher.update(&[0xffu8]);
                // hasher.update(&hex!("5FbDB2315678afecb367f032d93F642f64180aa3"));
                hasher.update(&deployer_clone);
                hasher.update(&buffer);
                hasher.update(&proxy_bytecode_hash_clone);
                hasher.finalize(&mut res);

                // final_addr = '0x' + Web3.solidity_keccak(
                //     ['bytes2', 'address', 'bytes1'],
                //     ['0xd694', Web3.to_checksum_address(proxy_addr), '0x01']
                // ).hex()[-40:]
                let mut hasher2 = Keccak::v256();
                hasher2.update(&[0xd6u8, 0x94u8]);
                hasher2.update(&res[12..32]);
                hasher2.update(&[0x01u8]);
                hasher2.finalize(&mut res);

                if  res[12] == prefix_clone[0] &&
                    hex::encode(&res[12..32]).starts_with(&prehex_clone)
                {
                    println!(
                        "Found address 0x{} with magic 0x{} in {} seconds after {}M iterations",
                        hex::encode(&res[12..32]),
                        hex::encode(&buffer[0..16]),
                        first.elapsed().as_secs(),
                        (index * args_threads as u64 / 1000) as f64 / 1000.0
                    );
                    std::process::exit(0);
                }
            }    
        })));
    }

    for i in 0..handles.len() {
        handles[i].take().map(std::thread::JoinHandle::join);
    }
}
