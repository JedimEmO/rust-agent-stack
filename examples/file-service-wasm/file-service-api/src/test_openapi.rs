#[cfg(test)]
mod tests {
    use crate::{generate_documentservice_openapi, generate_documentservice_openapi_to_file};

    #[test]
    fn test_openapi_function_exists() {
        // This test verifies that the OpenAPI generation function exists
        let _ = generate_documentservice_openapi();
        let _ = generate_documentservice_openapi_to_file();
    }
}
