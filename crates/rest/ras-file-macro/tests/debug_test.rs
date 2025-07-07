use ras_file_macro::file_service;

// Test to debug the parsing issue
fn main() {
    // This should expand the macro and show us any errors
    file_service!({
        service_name: TestService,
        base_path: "/test",
        endpoints: [
            UPLOAD UNAUTHORIZED test() -> (),
        ]
    });
}
