use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde_json::json;
use std::fs::File;
use std::io::Write;

const RPC_URL: &str = "http://127.0.0.1:18443";
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";
const RECEIVER_ADDRESS: &str = "bcrt1qq2yshcmzdlznnpxx258xswqlmqcxjs4dssfxt2";
const OP_RETURN_MESSAGE: &str = "We are all Satoshi!!";
const FEE_RATE: f64 = 21.0;

fn check_balance(rpc: &Client) -> bitcoincore_rpc::Result<f64> {
    let balance = rpc.get_balance(None, None)?.to_btc(); // Convert Amount to f64
    println!("Wallet Balance: {:.8} BTC", balance);
    Ok(balance)
}

fn main() -> bitcoincore_rpc::Result<()> {
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_string(), RPC_PASS.to_string()),
    )?;

    // Check Connection
    let info = rpc.get_blockchain_info()?;
    println!("{:?}", info);

    // Create or load the wallet
    match rpc.load_wallet("testwallet") {
        Ok(_) => println!("Loaded wallet: testwallet"),
        Err(_) => {
            // rpc.create_wallet("testwallet", None, None, None, None)?;
            println!("Created wallet: testwallet");
        }
    }

    // Generate a new address
    let wallet_address = rpc.get_new_address(None, None)?.assume_checked();

    println!("Generated address: {}", wallet_address);

    // Mine 101 blocks to the new address to activate the wallet with mined coins
    let _ = rpc.generate_to_address(101, &wallet_address)?;
    println!("Mined 101 blocks to wallet address");

    check_balance(&rpc)?;
    
    // Prepare a transaction to send 100 BTC
    let outputs = json!([
        { RECEIVER_ADDRESS: 100.0 }, // 100 BTC to the receiver
        {"data": hex::encode(OP_RETURN_MESSAGE.as_bytes())} // OP_RETURN output
    ]);

    let options = json!({"fee_rate": FEE_RATE});
    let raw_tx = rpc.call::<String>("createrawtransaction", &[json!([]), outputs])?;
    let funded_tx =
        rpc.call::<serde_json::Value>("fundrawtransaction", &[json!(raw_tx), options])?;
    let signed_tx =
        rpc.call::<serde_json::Value>("signrawtransactionwithwallet", &[funded_tx["hex"].clone()])?;

    // Send the transaction
    let tx_hex = signed_tx["hex"].as_str().unwrap();
    let txid = rpc.send_raw_transaction(tx_hex)?;

    // Write the txid to out.txt
    let mut file = File::create("out.txt")?;
    writeln!(file, "{}", txid)?;

    Ok(())
}
