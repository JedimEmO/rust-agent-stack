use crate::converter::OpenRpcToBrunoConverter;
use crate::error::ToolError;
use clap::Parser;
use std::path::PathBuf;

/// Convert OpenRPC specifications to Bruno API collections
#[derive(Parser)]
#[command(name = "openrpc-to-bruno")]
#[command(about = "Convert OpenRPC specifications to Bruno API collections")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Path to the OpenRPC specification file (JSON or YAML)
    #[arg(short, long, value_name = "FILE")]
    pub input: PathBuf,

    /// Output directory for the Bruno collection
    #[arg(short, long, value_name = "DIR")]
    pub output: PathBuf,

    /// Base URL for the API server (e.g., http://localhost:3000)
    #[arg(short, long, value_name = "URL")]
    pub base_url: Option<String>,

    /// Collection name (defaults to OpenRPC info.title)
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,

    /// Force overwrite existing Bruno collection
    #[arg(short, long)]
    pub force: bool,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub async fn run(&self) -> Result<(), ToolError> {
        if self.verbose {
            println!("🔍 Input file: {}", self.input.display());
            println!("📁 Output directory: {}", self.output.display());
            if let Some(base_url) = &self.base_url {
                println!("🌐 Base URL: {}", base_url);
            }
            if let Some(name) = &self.name {
                println!("📝 Collection name: {}", name);
            }
        }

        let converter = OpenRpcToBrunoConverter::new(self.clone());
        converter.convert().await
    }
}

impl Clone for Args {
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            output: self.output.clone(),
            base_url: self.base_url.clone(),
            name: self.name.clone(),
            force: self.force,
            verbose: self.verbose,
        }
    }
}
