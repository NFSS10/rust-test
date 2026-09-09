mod payment_engine;
mod utils;

use std::{env, io, process};

use crate::payment_engine::PaymentsEngine;
use crate::payment_engine::csv_utils::CsvTransactionRecord;
use crate::payment_engine::types::TransactionRecord;
use crate::utils::csv as csv_utils;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Error: Expected exactly one argument for the input CSV file.");
        eprintln!("Usage: cargo run -- myfile.csv > out.csv");
        process::exit(1);
    }

    let file_path = &args[0];

    let mut engine = PaymentsEngine::new();
    let result = csv_utils::with_csv_streaming::<CsvTransactionRecord, _>(file_path, |raw| {
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

    output_result(&engine);
}

fn output_result(engine: &PaymentsEngine) {
    let mut writer = ::csv::WriterBuilder::new().has_headers(true).from_writer(io::stdout());

    if let Err(err) = writer.write_record(["client", "available", "held", "total", "locked"]) {
        eprintln!("Error writing CSV header: {err}");
        process::exit(1);
    }

    let accounts_snapshot = engine.accounts_snapshot();
    for snapshot in accounts_snapshot {
        if let Err(err) = writer.write_record([
            snapshot.client_id.to_string(),
            format!("{:.4}", snapshot.available.round_dp(4)),
            format!("{:.4}", snapshot.held.round_dp(4)),
            format!("{:.4}", snapshot.total.round_dp(4)),
            snapshot.is_locked.to_string(),
        ]) {
            eprintln!("Error writing account snapshot: {err}");
            process::exit(1);
        }
    }

    if let Err(err) = writer.flush() {
        eprintln!("Error flushing CSV writer: {err}");
        process::exit(1);
    }
}
