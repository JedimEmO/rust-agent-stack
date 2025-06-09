use clap::Parser;

mod bruno;
mod cli;
mod converter;
mod error;

use crate::cli::Args;
use crate::error::ToolError;

#[tokio::main]
async fn main() -> Result<(), ToolError> {
    let args = Args::parse();

    match args.run().await {
        Ok(_) => {
            println!("✅ Successfully converted OpenRPC to Bruno collection");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
