// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_debugger::AptosDebugger;
use aptos_rest_client::Client;
use aptos_types::transaction::SignedTransaction;
use aptos_vm::AptosVM;
use clap::Parser;
use regex::Regex;
use std::io;
use url::Url;

#[derive(Parser)]
pub struct Argument {
    #[clap(long, default_value_t = false)]
    execute: bool,

    #[clap(long, default_value_t = 1)]
    concurrency_level: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    let re = Regex::new(r"\d+").unwrap();
    let bytes = re
        .find_iter(&buffer)
        .filter_map(|m| m.as_str().parse::<u8>().ok())
        .collect::<Vec<u8>>();

    let txn: SignedTransaction = bcs::from_bytes::<SignedTransaction>(&bytes)?;
    let chain_id = txn.chain_id();
    println!("===================");
    println!("Transaction Summary");
    println!("===================");
    println!("Sender: {:?}", txn.sender());
    println!("Sequence number: {:?}", txn.sequence_number());

    let network = if chain_id.is_mainnet() {
        "mainnet".to_string()
    } else if chain_id.is_testnet() {
        "testnet".to_string()
    } else {
        "devnet".to_string()
    };
    println!("Chain ID: {}", chain_id.id());
    println!("Network: {}", network);

    let endpoint = format!("https://{}.aptoslabs.com/v1", network);
    let debugger = AptosDebugger::rest_client(Client::new(Url::parse(&endpoint)?))?;
    let version = debugger
        .get_version_by_account_sequence(txn.sender(), txn.sequence_number())
        .await?
        .unwrap();
    println!("Version: {:?}", version);
    println!(
        "Overview: https://explorer.aptoslabs.com/txn/{:?}/userTxnOverview?network={}",
        version, network
    );
    println!(
        "Payload: https://explorer.aptoslabs.com/txn/{:?}/payload?network={}",
        version, network
    );

    let args = Argument::parse();
    if args.execute {
        aptos_logger::Logger::new().init();
        AptosVM::set_concurrency_level_once(args.concurrency_level);
        println!();
        println!("===============================");
        println!("Transaction re-execution result");
        println!("===============================");
        println!(
            "{:#?}",
            debugger.execute_past_transactions(version, 1).await?
        );
    }

    Ok(())
}
