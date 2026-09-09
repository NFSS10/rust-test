mod payment_engine;
mod utils;

use std::{env, process};

use crate::payment_engine::PaymentsEngine;
use crate::payment_engine::csv_utils::CsvTransactionRecord;
use crate::payment_engine::types::TransactionRecord;
use crate::utils::csv;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Error: Expected exactly one argument for the input CSV file.");
        eprintln!("Usage: cargo run -- myfile.csv > out.csv");
        process::exit(1);
    }

    let file_path = &args[0];

    let mut engine = PaymentsEngine::new();
    let result = csv::with_csv_streaming::<CsvTransactionRecord, _>(file_path, |raw| {
        let record: TransactionRecord = match raw.try_into() {
            Ok(r) => r,
            Err(err) => {
                eprintln!("Invalid CSV row: {err}");
                process::exit(1);
            }
        };

        engine.process_transaction(record).unwrap_or_else(|err| {
            eprintln!("Error processing transaction: {err}");
            process::exit(1);
        });
    });

    if let Err(err) = result {
        eprintln!("CSV error: {err}");
        process::exit(1);
    }
}
