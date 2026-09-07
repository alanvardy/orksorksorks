//! orksworksorks — conventions CLI.
//!
//! Ships the convention spine (typed errors, colored output, JSON envelope,
//! TOML config) behind the `orksworksorks init` command.

#![warn(missing_docs)]

mod errors;
mod format;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
