use oauth2_demo_api::*;
use std::fs;

fn main() {
    // Create target/openrpc directory if it doesn't exist
    fs::create_dir_all("openrpc").expect("Failed to create directory");

    // Generate OpenRPC document
    let openrpc_doc = generate_googleoauth2service_openrpc();

    println!("✅ OpenRPC document generated successfully!");
    println!("\n📋 Generated OpenRPC content:");
    println!("{}", serde_json::to_string_pretty(&openrpc_doc).unwrap());

    let file = generate_googleoauth2service_openrpc();
    fs::write(
        "openrpc/google-oauth2.openrpc.json",
        serde_json::to_string_pretty(&file).unwrap(),
    )
    .unwrap();
}
