//! # FR Data Downloader
//!
//! Utility to download FR Data from public API.
//!
//! The motivation to crate this tool was the need to conduct routine audits
//! on customer databases to ensure that all government requirements were met.
//!
//! Among the things that still need to be done are:
//!
//! - Correctly handle the requests with the API key;
//! - Generate the CSV summary from the downloaded data;
//! - Validate the FR;
//! - Separate results for multiple customers.

#[macro_use]
extern crate lazy_static;

use std::fs::{metadata, File};
use std::io::{BufRead, BufReader, Lines};

use filetime::FileTime;
use regex::Regex;
use walkdir::WalkDir;

// Added this macro to be able to have `static`s with data loaded from `dotenv` at runtime.
// Trying to use `const` in this case produces the following error:
// `calls in constants are limited to constant functions, tuple structs and tuple variants`
lazy_static! {
    /// URL to get data from.
    static ref API_URL: String = dotenv::var("API_URL").expect("Unable to get API URL.");

    /// Margin of error in seconds to get the data, respecting the limits of the API.
    static ref MARGIN_OF_ERROR: String =
        dotenv::var("MARGIN_OF_ERROR").unwrap_or_else(|_| "0".to_string());

    /// Limit of HTTP requests per minute according to the contracted plan.
    static ref LIMIT_PER_MINUTE: String =
        dotenv::var("LIMIT_PER_MINUTE").unwrap_or_else(|_| "3".to_string());

    /// Interval, in seconds, between each HTTP request according to the values specified
    /// in `LIMIT_PER_MINUTE` and `MARGIN_OF_ERROR`.
    static ref INTERVAL: f32 =
        60.0 / LIMIT_PER_MINUTE.parse::<f32>().unwrap() + MARGIN_OF_ERROR.parse::<f32>().unwrap();

    /// File containing the FR, separated by new line.
    static ref INPUT_FILE: String =
        dotenv::var("INPUT_FILE").unwrap_or_else(|_| "./input.txt".to_string());

    /// Path of the folder to save the data obtained from the API.
    static ref OUTPUT_FOLDER: String =
        dotenv::var("OUTPUT_FOLDER").unwrap_or_else(|_| "./downloads/".to_string());

    /// Maximum age of file to determine if it needs to be downloaded again.
    ///
    /// 30 days seems to be a good interval, since the FR data doesn't change
    /// so frequently, and this way we do not need to make so many requests to
    /// the server, since different customers may have associations with FRs
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

/// Read RFs from file line by line.
fn get_frs_from_file(file_name: &str) -> Lines<BufReader<File>> {
    BufReader::new(File::open(file_name).unwrap()).lines()
}

#[test]
fn frs_from_file() {
    use std::io::Write;
    let file_name = "test_frs";
    let mut file = File::create(file_name).unwrap();
    file.write_all(b"00000").unwrap();
    let content = get_frs_from_file(&file_name).nth(0).unwrap().unwrap();
    assert_eq!(content, "00000");
    std::fs::remove_file(&file_name).unwrap();
}

/// Remove all non-numeric characters from the FR so it can be used to make the
/// HTTP request to the API.
fn normalize_fr(fr: &str) -> String {
    Regex::new(r"[^0-9]")
        .unwrap()
        .replace_all(&fr, "")
        .to_string()
}

#[test]
fn normalized_frs() {
    assert_eq!(normalize_fr(""), "");
    assert_eq!(normalize_fr("12"), "12");
    assert_eq!(normalize_fr("no numbers"), "");
    assert_eq!(normalize_fr(" as-12.df "), "12");
}

/// Check if the specified FR already has the respective file in the `OUTPUT_FOLDER`.
fn is_downloaded(fr: &str) -> bool {
    for entry in WalkDir::new(OUTPUT_FOLDER.to_string()) {
        let path = entry.unwrap().path().to_str().unwrap().to_owned();
        if path.contains(fr) {
            return true;
        }
    }
    false
}

#[test]
fn downloads() {
    let file_name = "test_download";
    let file_path = format!("{}{}", *OUTPUT_FOLDER, file_name);
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

#[doc(hidden)]
fn main() {
    create_output_folder(OUTPUT_FOLDER.as_str());
    for fr in get_frs_from_file(INPUT_FILE.as_str()) {
        let normalized_fr = normalize_fr(&fr.unwrap());
        let api_call = format!("{}{}", API_URL.to_string(), normalized_fr);
        let file_path = format!("{}{}.json", OUTPUT_FOLDER.to_string(), normalized_fr);

        println!("{:?}", is_downloaded(&normalized_fr));
        println!("{:?}", is_old(get_age_of_file(&file_path)));
    }
}
