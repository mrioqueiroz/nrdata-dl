# nrdata-dl

## NR Data Downloader

Utility to download NR Data from public API.

*This is still a work in progress, so not all the functionalities are
available.*

To know more, please take a look at the comments in
[`src/main.rs`](src/main.rs).

#### Motivation

The motivation to create this tool was the need to conduct routine audits
on customer databases to ensure that all government requirements were met
before the respective deadlines.

#### Building

If using NixOS, just run `nix-shell --pure` to build the development
environment. On Ubuntu, remember to install `libssl-dev`.

#### Roadmap

Among the things that still need to be done are:

- Correctly handle the requests with the API key;
- Reorder functions by usage to improve readability;
- Correctly handle errors (remove `.unwrap()`);
- Generate the CSV summary from the downloaded data;
- Validate the NR;
- Generate logs;
- Get data from command-line arguments (having priority over the .env file);
- Separate results for multiple customers.
  - This can be done by creating a `.zip` file containing only the downloaded
    files that are in the current input list.
