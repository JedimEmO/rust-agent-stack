// Test parsing directly
use ras_file_macro::file_service;

// This should work - no path params
file_service!({
    service_name: Test1,
    base_path: "/api",
    endpoints: [
        UPLOAD UNAUTHORIZED upload() -> (),
    ]
});

// This should also work - with path params
file_service!({
    service_name: Test2,
    base_path: "/api",
    endpoints: [
        DOWNLOAD UNAUTHORIZED download/{id: String}() -> (),
    ]
});

#[test]
fn test_parse() {
    // If this compiles, parsing works
}
