//! # NR Data Downloader
//!
//! Utility to download NR Data from public API.
//!
//! *This is still a work in progress, so not all the functionalities are
//! available.*
//!
//! To know more, please take a look at the comments in
//! [`src/main.rs`](src/main.rs).
//!
//! ### Motivation
//!
//! The motivation to create this tool was the need to conduct routine audits
//! on customer databases to ensure that all government requirements were met
//! before the respective deadlines.
//!
//! ### Building
//!
//! If using NixOS, just run `nix-shell --pure` to build the development
//! environment. On Ubuntu, remember to install `libssl-dev`.
//!
//! ### Roadmap
//!
//! Among the things that still need to be done are:
//!
//! - Correctly handle the requests with the API key;
//! - Reorder functions by usage to improve readability;
//! - Correctly handle errors (remove `.unwrap()`);
//! - Generate the CSV summary from the downloaded data;
//! - Validate the NR;
//! - Generate logs;
//! - Get data from command-line arguments (having priority over the .env file);
//! - Separate results for multiple customers.
//!   - This can be done by creating a `.zip` file containing only the downloaded
//!     files that are in the current input list.

#[macro_use]
extern crate lazy_static;

use std::fs::{metadata, File};
use std::io::{BufRead, BufReader, Lines, Write};
use std::{thread, time};

use filetime::FileTime;
use regex::Regex;
use walkdir::WalkDir;

// Added this macro to be able to have `static`s with data loaded from `dotenv`
// at runtime. Trying to use `const` in this case produces the following error:
// `calls in constants are limited to constant functions, tuple structs and
// tuple variants`
lazy_static! {
    /// URL to get data from.
    static ref API_URL: String = dotenv::var("API_URL").expect("Unable to get API URL.");

    /// Margin of error (in seconds) to get the data, respecting the limits of the API.
    static ref MARGIN_OF_ERROR: String =
        dotenv::var("MARGIN_OF_ERROR").unwrap_or_else(|_| "0".to_string());

    /// Limit of HTTP requests per minute according to the contracted plan.
    static ref LIMIT_PER_MINUTE: String =
        dotenv::var("LIMIT_PER_MINUTE").unwrap_or_else(|_| "3".to_string());

    /// Interval (in seconds) between each HTTP request, based on the values specified
    /// in `LIMIT_PER_MINUTE` and `MARGIN_OF_ERROR`.
    static ref INTERVAL: f32 =
        60.0 / LIMIT_PER_MINUTE.parse::<f32>().unwrap() + MARGIN_OF_ERROR.parse::<f32>().unwrap();

    /// File containing the NRs. The NRs must be separated by new line.
    static ref INPUT_FILE: String =
        dotenv::var("INPUT_FILE").unwrap_or_else(|_| "./input.txt".to_string());

    /// Path of the folder to save the data obtained from the API.
    ///
    /// If the folder already contains data related to any of the NRs from
    /// the input file, and they are not older than the specified days, the
    /// data will not be downladed again.
    static ref OUTPUT_FOLDER: String =
        dotenv::var("OUTPUT_FOLDER").unwrap_or_else(|_| "./downloads/".to_string());

    /// Maximum age of file to determine if it needs to be downloaded again.
    ///
    /// 30 days seems to be a good interval, since the NR data doesn't change
    /// so frequently, and this way we do not need to make so many requests to
    /// the server, since different customers may have associations with NRs
    /// from others.
    static ref MAXIMUM_AGE: i64 =
        dotenv::var("MAXIMUM_AGE").unwrap_or_else(|_| "30".to_string()).parse::<i64>().unwrap();
}

/// Create output folder in the current directory if not exists.
/// Do nothing otherwise.
fn create_output_folder(folder_name: &str) {
    if std::fs::create_dir(folder_name).is_ok() {}
}

#[test]
fn output_folder_creation_and_deletion() {
    let folder_name = "test/";
    create_output_folder(&folder_name);
    assert_eq!(std::path::Path::exists((&folder_name).as_ref()), true);
    std::fs::remove_dir(&folder_name).unwrap();
    assert_eq!(std::path::Path::exists((&folder_name).as_ref()), false);
}

/// Return the NRs from the input file.
fn get_nrs_from_file(file_name: &str) -> Lines<BufReader<File>> {
    BufReader::new(File::open(file_name).unwrap()).lines()
}

#[test]
fn nrs_from_file() {
    let file_name = "test_nrs";
    let mut file = File::create(file_name).unwrap();
    file.write_all(b"00000").unwrap();
    let content = get_nrs_from_file(&file_name).nth(0).unwrap().unwrap();
    assert_eq!(content, "00000");
    std::fs::remove_file(&file_name).unwrap();
}

/// Remove all non-numeric characters from the NR so it can be used to make the
/// HTTP request to the API no matter the format the user specify in the
/// input file.
fn normalize_nr(nr: &str) -> String {
    Regex::new(r"[^0-9]")
        .unwrap()
        .replace_all(&nr, "")
        .to_string()
}

#[test]
fn normalized_nrs() {
    assert_eq!(normalize_nr(""), "");
    assert_eq!(normalize_nr("12"), "12");
    assert_eq!(normalize_nr("no numbers"), "");
    assert_eq!(normalize_nr(" as-12.df "), "12");
}

/// Check if the specified NR already has the respective file in the `OUTPUT_FOLDER`.
fn is_downloaded(nr: &str) -> bool {
    for entry in WalkDir::new(OUTPUT_FOLDER.to_string()) {
        let path = entry.unwrap().path().to_str().unwrap().to_owned();
        if path.contains(nr) {
            return true;
        }
    }
    false
}

#[test]
fn downloads() {
    let file_name = "test_download";
    let file_path = format!("{}{}", *OUTPUT_FOLDER, file_name);
    std::fs::create_dir_all(OUTPUT_FOLDER.to_string()).unwrap();
    File::create(&file_path).unwrap();
    assert_eq!(is_downloaded(&file_name), true);
    std::fs::remove_file(&file_path).unwrap();
    assert_eq!(is_downloaded(&file_name), false);
}

/// Check if the downloaded file is older than the specified `MAXIMUM_AGE`.
/// If so, it needs to be downloaded again.
fn is_old(age_of_file: i64) -> bool {
    age_of_file > *MAXIMUM_AGE
}

#[test]
fn test_is_old() {
    if *MAXIMUM_AGE == 30 {
        assert_eq!(is_old(1), false);
        assert_eq!(is_old(30), false);
        assert_eq!(is_old(31), true);
    }
}

/// Get the age of the file as day.
fn get_age_of_file(file_name: &str) -> i64 {
    let metadata = metadata(file_name).unwrap();

    // Here we are getting the modification date because, as the `filetime`
    // documentation, _not all Unix platforms have this field available and
    // may return None in some circumstances_.
    age_in_days(
        FileTime::now().seconds() - FileTime::from_last_modification_time(&metadata).seconds(),
    )
}

#[test]
fn age_of_new_file() {
    let file_name = "test_age";
    let file_path = format!("{}{}", *OUTPUT_FOLDER, file_name);
    std::fs::create_dir_all(OUTPUT_FOLDER.to_string()).unwrap();
    File::create(&file_path).unwrap();
    assert_eq!(get_age_of_file(&file_path), 0);
    std::fs::remove_file(&file_path).unwrap();
}

/// Helper function to convert the timestamp as day.
fn age_in_days(seconds: i64) -> i64 {
    let age_in_minutes = seconds / 60;
    let age_in_hours = age_in_minutes / 60;
    age_in_hours / 24
}

#[test]
fn test_age_in_days() {
    let sec_day = 86400;
    assert_eq!(age_in_days(sec_day - 100), 0);
    assert_eq!(age_in_days(sec_day), 1);
    assert_eq!(age_in_days(sec_day + 100), 1);
    assert_eq!(age_in_days(sec_day * 2), 2);
    assert_eq!(age_in_days(sec_day * 2 + 100), 2);
}

/// Make the actual request to the API.
///
/// Since the API limits the number of requests per minute, there is no need
/// to use `async` at this time.
fn make_request(url: &str) -> String {
    for _ in &[..3] {
        println!("Waiting for response from API...");
        let start_time = std::time::Instant::now();
        let response = reqwest::blocking::get(url);
        if let Err(e) = response {
            if e.is_timeout() {
                println!("Timed out. Retrying...");
                thread::sleep(time::Duration::from_secs(2));
                continue;
            }
        } else if let Ok(r) = response {
            if r.status().as_str() == "200" {
                println!("Data received.");
                let duration = start_time.elapsed().as_secs_f32();
                if duration < *INTERVAL {
                    let interval = *INTERVAL - duration;
                    println!("Waiting {} seconds before next action...", interval);
                    thread::sleep(time::Duration::from_secs(interval as u64));
                }
                return r.text().unwrap();
            }
        }
    }
    println!("Got nothing...");
    String::from("")
}

#[doc(hidden)]
fn main() {
    create_output_folder(OUTPUT_FOLDER.as_str());
    for nr in get_nrs_from_file(INPUT_FILE.as_str()) {
        let normalized_nr = normalize_nr(&nr.unwrap());
        let api_call = format!("{}{}", API_URL.to_string(), normalized_nr);
        let file_path = format!("{}{}.json", OUTPUT_FOLDER.to_string(), normalized_nr);
        // TODO: Check if file contains valid data.
        if !is_downloaded(&normalized_nr)
            | (is_downloaded(&normalized_nr) && is_old(get_age_of_file(&file_path)))
        {
            println!("Requesting {} data...", normalized_nr);
            let nr_data = make_request(&api_call);
            if nr_data != *"" {
                let mut nr_file = File::create(&file_path).unwrap();
                nr_file.write_all(&nr_data.as_bytes()).unwrap();
            }
        } else {
            println!("Skipping {}. Already saved...", normalized_nr);
        }
    }
    println!("All done.")
}
