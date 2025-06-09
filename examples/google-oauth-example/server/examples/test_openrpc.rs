// Simple test to verify OpenRPC generation works
fn main() {
    use google_oauth_example::api::*;

    // Create target/openrpc directory if it doesn't exist
    std::fs::create_dir_all("target/openrpc").expect("Failed to create directory");

    // Generate OpenRPC document
    let openrpc_doc = generate_googleoauth2service_openrpc();

    println!("✅ OpenRPC document generated successfully!");
    println!("\n📋 Generated OpenRPC content:");
    println!("{}", serde_json::to_string_pretty(&openrpc_doc).unwrap());

    // Also write to file
    match generate_googleoauth2service_openrpc_to_file() {
        Ok(()) => {
            println!("\n📄 OpenRPC document written to: target/openrpc/googleoauth2service.json");
        }
        Err(e) => {
            eprintln!("\n❌ Failed to write OpenRPC document: {}", e);
        }
    }
}
