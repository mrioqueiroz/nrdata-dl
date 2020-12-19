//! # FR Data Downloader
//!
//! Utility to download FR Data in batch.

#[macro_use]
extern crate lazy_static;

use dotenv;

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
}

fn main() {

}
