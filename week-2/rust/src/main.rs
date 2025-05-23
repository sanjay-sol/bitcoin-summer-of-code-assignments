use bitcoin::blockdata::script::{Builder, ScriptBuf};
use bitcoin::blockdata::transaction::{Transaction, TxIn, TxOut};
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::{sha256d, Hash};
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::sighash::SighashCache;
use bitcoin::transaction::Version;
use bitcoin::{Address, Amount, EcdsaSighashType, Network, Txid};
use bitcoin::{OutPoint, Witness};
use std::fs::File;
use std::io::Write;
use std::str::FromStr;

fn main() {
    let secp_ctx = Secp256k1::new();

    let key_one = SecretKey::from_slice(
        &hex::decode("39dc0a9f0b185a2ee56349691f34716e6e0cda06a7f9707742ac113c4e2317bf").unwrap(),
    )
    .unwrap();
    let key_two = SecretKey::from_slice(
        &hex::decode("5077ccd9c558b7d04a81920d38aa11b4a9f9de3b23fab45c3ef28039920fdd6d").unwrap(),
    )
    .unwrap();

    let multisig_script = ScriptBuf::from(
        hex::decode(
            "5221032ff8c5df0bc00fe1ac2319c3b8070d6d1e04cfbf4fedda499ae7b775185ad53b21039bbc8d24f89e5bc44c5b0d1980d6658316a6b2440023117c3c03a4975b04dd5652ae",
        )
        .unwrap(),
    );

    let script_hash = multisig_script.wscript_hash();
    let redeem_script = ScriptBuf::new_p2wsh(&script_hash);
    let redeem_bytes: [u8; 34] = redeem_script
        .as_bytes()
        .try_into()
        .expect("Invalid redeem script length");

    let script_sig = Builder::new().push_slice(redeem_bytes).into_script();

    let prev_transaction_id = Txid::from_raw_hash(sha256d::Hash::from_byte_array([0; 32]));
    let input_data = TxIn {
        previous_output: OutPoint {
            txid: prev_transaction_id,
            vout: 0,
        },
        script_sig: script_sig,
        sequence: bitcoin::Sequence(0xffffffff),
        witness: Witness::new(),
    };

    let output_data = TxOut {
        value: Amount::from_sat(100_000),
        script_pubkey: Address::from_str("325UUecEQuyrTd28Xs2hvAxdAjHM7XzqVF").unwrap()
            .require_network(Network::Bitcoin)
            .unwrap()
            .script_pubkey(),
    };

    let mut transaction = Transaction {
        version: Version(2),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![input_data],
        output: vec![output_data],
    };

    let mut sig_hash_cache = SighashCache::new(&transaction);
    let sighash_val = sig_hash_cache
        .p2wsh_signature_hash(
            0,
            &multisig_script,
            Amount::from_sat(100_000),
            EcdsaSighashType::All,
        )
        .unwrap();

    let msg_data = Message::from_digest(*sighash_val.as_ref());
    let signature_one = secp_ctx.sign_ecdsa(&msg_data, &key_one);
    let signature_two = secp_ctx.sign_ecdsa(&msg_data, &key_two);

    let mut sig1_bytes = signature_one.serialize_der().to_vec();
    sig1_bytes.push(EcdsaSighashType::All as u8);
    let mut sig2_bytes = signature_two.serialize_der().to_vec();
    sig2_bytes.push(EcdsaSighashType::All as u8);

    let witness_stack = &mut transaction.input[0].witness;
    witness_stack.push(vec![]);
    witness_stack.push(sig2_bytes);
    witness_stack.push(sig1_bytes);
    witness_stack.push(multisig_script.to_bytes());

    let transaction_hex = serialize_hex(&transaction);
    let mut file = File::create("out.txt").unwrap();
    file.write_all(transaction_hex.as_bytes()).unwrap();

    println!("Final Transaction Hex: {}", transaction_hex);
}