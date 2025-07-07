use ras_file_macro::file_service;

// Simplest possible test
file_service!({
    service_name: SimpleService,
    base_path: "/api",
    endpoints: [
        UPLOAD UNAUTHORIZED upload() -> (),
    ]
});

#[test]
fn test_compilation() {
    // If it compiles, the test passes
    assert!(true);
}
