use bitcoin::{consensus::deserialize, Transaction};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Input {
    pub txid: String,
    pub vout: u32,
    pub sequence: u32,
    pub prevout: Option<PrevOutType>,
    pub is_coinbase: bool,
    pub scriptsig: String,
    pub scriptsig_asm: String,
    pub witness: Option<Vec<String>>,
    pub inner_redeemscript_asm: Option<String>,
    pub inner_witnessscript_asm: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrevOutType {
    pub value: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Output {
    pub value: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub vin: Vec<Input>,
    pub vout: Vec<Output>,
    pub weight: u32,
    pub fee: u32,
    pub hex: String,
}

pub fn calculate_total_fee(blockchain_data: &[BitcoinTransaction]) -> u64 {
    blockchain_data
        .iter()
        .map(|tx| {
            let spent_sum: u64 = tx
                .vin
                .iter()
                .filter_map(|input| input.prevout.as_ref().map(|prev| prev.value))
                .sum();

            let created_sum: u64 = tx.vout.iter().map(|output| output.value).sum();

            spent_sum.saturating_sub(created_sum) // Ensuring no underflow
        })
        .sum()
}

pub struct BlockHeader {
    pub version: Vec<u8>,
    pub prev_hash: Vec<u8>,
    pub merkle_root: Vec<u8>,
    pub timestamp: Vec<u8>,
    pub difficulty: Vec<u8>,
}

impl BlockHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(80);
        bytes.extend_from_slice(&self.version);
        bytes.extend_from_slice(&self.prev_hash);
        bytes.extend_from_slice(&self.merkle_root);
        bytes.extend_from_slice(&self.timestamp);
        bytes.extend_from_slice(&self.difficulty);
        bytes
    }
}
pub fn bytewise_comparator(seq1: &[u8], seq2: &[u8]) -> i32 {
    let mut iter1 = seq1.iter();
    let mut iter2 = seq2.iter();

    while let (Some(&b1), Some(&b2)) = (iter1.next(), iter2.next()) {
        match b1.cmp(&b2) {
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Greater => return 1,
            _ => continue,
        }
    }
    0
}
