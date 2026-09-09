use csv::{ReaderBuilder, Trim};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;

/// Buffer size for the CSV reader.
///
/// The bigger the buffer, the more memory it will use, but it can improve performance for large CSV files
/// as it reduces the number of I/O operations.
const CSV_READER_BUFFER_SIZE: usize = 256 * 1024; // 256 KB

/// Custom error type for CSV streaming operations.
#[derive(Debug)]
pub enum CsvStreamError {
    /// Represents an I/O error that occurred while reading the CSV file.
    Io(std::io::Error),
    /// Represents an error produced by the `csv` crate while reading records or deserializing a row into the target type `T`.
    Csv(csv::Error),
}
impl fmt::Display for CsvStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsvStreamError::Io(err) => write!(f, "I/O error while reading CSV: {err}"),
            CsvStreamError::Csv(err) => write!(f, "CSV parse error: {err}"),
        }
    }
}
impl Error for CsvStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CsvStreamError::Io(err) => Some(err),
            CsvStreamError::Csv(err) => Some(err),
        }
    }
}
impl From<std::io::Error> for CsvStreamError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
impl From<csv::Error> for CsvStreamError {
    fn from(err: csv::Error) -> Self {
        Self::Csv(err)
    }
}

/// Streams a CSV file by deserializing each row into a struct of type `T` and passing it to the provided callback function `on_row`.
///
/// NOTE: It assumes that the CSV file has headers.
pub fn with_csv_streaming<T, F>(path: &str, mut on_row: F) -> Result<(), CsvStreamError>
where
    T: DeserializeOwned,
    F: FnMut(T),
{
    let file = File::open(path)?;

    let reader = BufReader::with_capacity(CSV_READER_BUFFER_SIZE, file);
    let mut csv = ReaderBuilder::new()
        .has_headers(true) // Assume the CSV file has headers
        .flexible(true) // Allow rows with different numbers of fields
        .trim(Trim::All) // Trim whitespace from all fields
        .from_reader(reader);

    for row in csv.deserialize::<T>() {
        on_row(row?);
    }

    Ok(())
}
