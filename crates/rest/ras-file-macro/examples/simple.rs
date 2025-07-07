use ras_file_macro::file_service;

file_service!({
    service_name: SimpleService,
    base_path: "/api",
    endpoints: [
        UPLOAD UNAUTHORIZED upload() -> (),
    ]
});

fn main() {
    println!("Compiled!");
}
