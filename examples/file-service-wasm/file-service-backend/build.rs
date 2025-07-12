fn main() {
    println!("cargo:rerun-if-changed=../file-service-api/src/lib.rs");

    // Generate OpenAPI spec using the file service API
    match file_service_api::generate_documentservice_openapi_to_file() {
        Ok(_) => println!("Generated OpenAPI specification"),
        Err(e) => eprintln!("Failed to generate OpenAPI spec: {}", e),
    }
}
