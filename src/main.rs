//! # FR Data Downloader
//!
//! Utility to download FR Data in batch.

#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::{BufRead, BufReader, Lines};

use dotenv;
use regex::Regex;

// Added this macro to be able to have `static`s with data loaded from `dotenv` at runtime.
// Trying to use `const` in this case produces the following error:
// `calls in constants are limited to constant functions, tuple structs and tuple variants`
lazy_static! {
    static ref API_URL: String = dotenv::var("API_URL").expect("Unable to get API URL.");
    static ref MARGIN_OF_ERROR: String = dotenv::var("MARGIN_OF_ERROR").unwrap_or("0".to_string());
    static ref LIMIT_PER_MINUTE: String =
        dotenv::var("LIMIT_PER_MINUTE").unwrap_or("3".to_string());
    static ref INTERVAL: f32 =
        60.0 / LIMIT_PER_MINUTE.parse::<f32>().unwrap() + MARGIN_OF_ERROR.parse::<f32>().unwrap();
    static ref INPUT_FILE: String = dotenv::var("INPUT_FILE").unwrap_or("./input.txt".to_string());
}

fn save(name: &str, data: &str) {
    match std::fs::create_dir(".downloads/") {
        Ok(_) => println!("Creating directory."),
        Err(_) => println!("Directory already exists."),
    }
}

fn get_lines_from_file() -> Lines<BufReader<File>> {
    match file = File::open(INPUT_FILE.as_str()) {
        Some(file) => file,
        None => eprintln!("Unable to open the file."),
    };
    let reader = BufReader::new(file);
    reader.lines()
}

fn normalize_fr(fr: &str) -> String {
    //! Remove all non-numeric characters from the FR.
    let regex = Regex::new(r"[^0-9]").unwrap();
    regex.replace_all(&fr, "").to_string()
}

fn main() {
    for line in get_lines_from_file() {
        println!("{:?}", normalize_fr(&line.unwrap()));
    }
}
