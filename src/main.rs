use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Error: Expected exactly one argument for the input CSV file.");
        eprintln!("Usage: cargo run -- myfile.csv > out.csv");
        process::exit(1);
    }

    let file_path = &args[0];
    println!("Processing file: {}", file_path);
}
