use bitcoin::Transaction;
use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};
use utils::{bytewise_comparator, calculate_total_fee, BitcoinTransaction, BlockHeader};
mod utils;
mod utxo;
use bitcoin::{
    absolute::LockTime,
    blockdata::{
        script::ScriptBuf,
        transaction::{TxIn, TxOut},
    },
    consensus::{deserialize, encode::serialize},
    transaction::Version,
    OutPoint, Sequence, Witness,
};

use sha2::{Digest, Sha256};
pub const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub fn reverse(hex_str: &str) -> Result<String, hex::FromHexError> {
    hex::decode(hex_str).map(|mut bytes| {
        bytes.reverse();
        hex::encode(bytes)
    })
}

fn sha256_hex(input: &str) -> String {
    let decoded = hex::decode(input).expect("Invalid hex input");
    let hash = Sha256::digest(&decoded);
    hex::encode(hash)
}

fn hash_twice(input: &str) -> String {
    let decoded_bytes = hex::decode(input).expect("Invalid hex input");

    let first_hash = Sha256::digest(&decoded_bytes);
    let second_hash = Sha256::digest(first_hash);

    hex::encode(second_hash)
}
fn calc_merkle_root(mut transactions: Vec<String>) -> String {
    if transactions.is_empty() {
        return String::new(); // Return empty if no transactions
    }

    while transactions.len() > 1 {
        if transactions.len() % 2 == 1 {
            transactions.push(transactions.last().unwrap().clone()); // Duplicate last if odd
        }

        transactions = transactions
            .chunks(2)
            .map(|pair| {
                let concatenated = pair[0].clone() + &pair[1];
                let hash_once = sha256_hex(&concatenated);
                sha256_hex(&hash_once) // Double SHA-256
            })
            .collect();
    }

    transactions[0].clone()
}

fn compute_commitment_hash(tx_list: &[BitcoinTransaction]) -> String {
    let mut witness_hashes = Vec::with_capacity(tx_list.len());

    for tx in tx_list {
        let raw_data = hex::decode(&tx.hex).expect("Invalid hex input");
        let parsed_tx: Transaction = deserialize(&raw_data).expect("Failed to parse transaction");
        let witness_id = parsed_tx.compute_wtxid().to_string();

        // Reverse the witness transaction ID
        let reversed_id = reverse(&witness_id).expect("Failed to reverse hex");
        witness_hashes.push(reversed_id);
    }

    let mut merkle_nodes =
        vec!["0000000000000000000000000000000000000000000000000000000000000000".to_string()];
    merkle_nodes.extend(witness_hashes);

    let witness_merkle_root = calc_merkle_root(merkle_nodes);
    let final_commitment_hash = hash_twice(&format!(
        "{}0000000000000000000000000000000000000000000000000000000000000000",
        witness_merkle_root
    ));

    final_commitment_hash
}

pub fn craft_reward_transaction(fee_sum: u64, header_commit: &str) -> (String, String) {
    let scriptsig_parts: [&str; 4] = ["03", "23", "37", "08"];
    let first_output_parts: [&str; 10] =
        ["76", "a9", "14", "ed", "f1", "0a", "7f", "ac", "6b", "32"];

    let op_return_prefix_parts: [&str; 4] = ["6a", "24", "aa", "21"];

    let script_sig_data =
        ScriptBuf::from(hex::decode(scriptsig_parts.concat()).expect("Malformed hex in scriptsig"));

    let witness_data =
        vec![
            hex::decode("0000000000000000000000000000000000000000000000000000000000000000")
                .expect("Malformed hex in witness"),
        ];

    let first_output_script = ScriptBuf::from(
        hex::decode(first_output_parts.concat()).expect("Malformed hex in first output script"),
    );

    let second_output_script_hex = format!(
        "{}{}{}",
        op_return_prefix_parts.concat(),
        "a9ed",
        header_commit
    );
    let second_output_script = ScriptBuf::from(
        hex::decode(&second_output_script_hex).expect("Malformed hex in second output script"),
    );

    let constructed_tx = Transaction {
        version: Version(1),
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: script_sig_data,
            sequence: Sequence::MAX,
            witness: Witness::from(witness_data),
        }],
        output: vec![
            TxOut {
                value: bitcoin::Amount::from_sat(fee_sum + 1250000000),
                script_pubkey: first_output_script,
            },
            TxOut {
                value: bitcoin::Amount::ZERO,
                script_pubkey: second_output_script,
            },
        ],
    };

    let tx_serialized = serialize(&constructed_tx);
    let tx_serialized_hex = hex::encode(tx_serialized);
    let tx_identifier = sha256_hex(&sha256_hex(&tx_serialized_hex));

    (tx_identifier, tx_serialized_hex)
}
pub fn proof_of_work_recursive(header: &BlockHeader, counter: u32) -> String {
    let block_header = header.to_bytes();

    let difficulty_threshold =
        hex::decode("0000ffff00000000000000000000000000000000000000000000000000000000")
            .expect("Malformed difficulty hex");

    let nonce_bytes = counter.to_le_bytes();

    let mut candidate_header = block_header.clone();
    candidate_header.extend_from_slice(&nonce_bytes);

    let hash_round1 = Sha256::digest(&candidate_header);
    let hash_final = Sha256::digest(&hash_round1);

    let mut processed_hash = hash_final.to_vec();
    processed_hash.reverse();

    if bytewise_comparator(&difficulty_threshold, &processed_hash) >= 0 {
        let mut completed_block = block_header;
        completed_block.extend_from_slice(&nonce_bytes);
        return hex::encode(completed_block);
    }

    proof_of_work_recursive(header, counter.wrapping_add(1))
}

pub fn proof_of_work(header: &BlockHeader) -> String {
    proof_of_work_recursive(header, 0)
}
fn main() {
    let mempool_dir = Path::new("mempool");

    let mut transactions = Vec::new();

    if let Ok(entries) = fs::read_dir(mempool_dir) {
        for entry in entries.flatten() {
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                match serde_json::from_str::<BitcoinTransaction>(&contents) {
                    Ok(tx) => transactions.push(tx),
                    Err(_) => println!("invalid tx "),
                }
            }
        }
    } else {
        println!("error");
    }

    let all_transaction = transactions.clone();
    let valid_utxos = utxo::filter_valid_transactions(&all_transaction);
    let total_fee = calculate_total_fee(&valid_utxos);

    let commitment_hash = compute_commitment_hash(&valid_utxos);
    let (coinbase_tx_id, serialized_tx_hex) =
        craft_reward_transaction(total_fee, commitment_hash.as_str());

    let merkle_root_leaves: Vec<String> =
        std::iter::once(reverse(&coinbase_tx_id).expect("Failed to reverse coinbase"))
            .chain(
                valid_utxos
                    .iter()
                    .map(|tx| reverse(&tx.txid).expect("Failed to reverse tx hex")),
            )
            .collect();

    let merkle_root = calc_merkle_root(merkle_root_leaves);

    let version: u32 = 4;
    let version_bytes = version.to_le_bytes();

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as u32;

    let time_bytes = time.to_le_bytes();
    let nbits_bytes = (0x1f00ffff as u32).to_le_bytes();

    let prev_block_hash_bytes =
        hex::decode("0000ffff00000000000000000000000000000000000000000000000000000000")
            .expect("Invalid hex in previous block hash");

    let merkle_root_bytes = hex::decode(&merkle_root).expect("Invalid hex in merkle root");

    let block = BlockHeader {
        version: version_bytes.to_vec(),
        prev_hash: prev_block_hash_bytes.to_vec(),
        merkle_root: merkle_root_bytes.to_vec(),
        timestamp: time_bytes.to_vec(),
        difficulty: nbits_bytes.to_vec(),
    };

    let block_header_hex = proof_of_work(&block);

    let mut output_file = File::create("out.txt").expect("Could not create output file");
    output_file
        .write_all(block_header_hex.as_bytes())
        .expect("Error writing block hash");
    output_file.write_all(b"\n").unwrap();
    output_file
        .write_all(serialized_tx_hex.as_bytes())
        .expect("Error writing coinbase hex");
    output_file.write_all(b"\n").unwrap();
    output_file
        .write_all(coinbase_tx_id.as_bytes())
        .expect("Error writing coinbase hash");
    output_file.write_all(b"\n").unwrap();

    for transaction in valid_utxos.iter() {
        output_file
            .write_all(transaction.txid.as_bytes())
            .expect("Error writing transaction ID");
        output_file.write_all(b"\n").unwrap();
    }
}
