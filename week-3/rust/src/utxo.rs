use crate::utils::BitcoinTransaction;
use std::collections::HashSet;

pub fn filter_valid_transactions(tx_pool: &[BitcoinTransaction]) -> Vec<BitcoinTransaction> {
    let mut spent_outputs = Vec::new();
    let mut valid_tx_ids = HashSet::new();

    for transaction in tx_pool {
        for input in &transaction.vin {
            let spent_key = format!("{}{}", input.txid, input.vout);
            spent_outputs.push((spent_key, transaction.txid.clone()));
        }
    }

    for (_, txid) in &spent_outputs {
        valid_tx_ids.insert(txid.clone());
    }

    let mut sorted_transactions: Vec<BitcoinTransaction> = tx_pool
        .iter()
        .filter(|tx| valid_tx_ids.contains(&tx.txid))
        .cloned()
        .collect();

    // Sort transactions by fee rate (higher is better)
    sorted_transactions.sort_unstable_by(|tx1, tx2| {
        (tx2.fee as f64 / tx2.weight as f64)
            .partial_cmp(&(tx1.fee as f64 / tx1.weight as f64))
            .unwrap()
    });

    let mut selected_transactions = Vec::with_capacity(sorted_transactions.len());
    let mut total_weight = 0;
    const MAX_BLOCK_WEIGHT: u32 = 4_000_000;

    for transaction in sorted_transactions {
        if total_weight + transaction.weight > MAX_BLOCK_WEIGHT {
            break;
        }
        total_weight += transaction.weight;
        selected_transactions.push(transaction);
    }

    selected_transactions
}
