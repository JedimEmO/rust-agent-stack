#![allow(dead_code)]

use ras_file_macro::file_service;

file_service!({
    service_name: TestService,
    base_path: "/api",
    endpoints: [
        UPLOAD UNAUTHORIZED test() -> (),
    ]
});
