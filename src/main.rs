mod payment_engine;
mod utils;

use std::{env, process};

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

    let result = csv::with_csv_streaming::<TransactionRecord, _>(file_path, |record| {
        println!("Processed record: {:?}", record);
    });

    if let Err(err) = result {
        eprintln!("CSV error: {err}");
        process::exit(1);
    }
}
