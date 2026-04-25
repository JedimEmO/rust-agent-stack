use std::path::Path;
use tempfile::tempdir;
use tokio::fs;

async fn test_conversion(
    input_file: &str,
    expected_methods: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let input_path = Path::new("tests/fixtures").join(input_file);
    let output_dir = tempdir()?;

    // Run the conversion
    let args = vec![
        "openrpc-to-bruno",
        "--input",
        input_path.to_str().unwrap(),
        "--output",
        output_dir.path().to_str().unwrap(),
        "--force",
    ];

    // For testing, we'll directly test the conversion logic
    use clap::Parser;
    use openrpc_to_bruno::cli::Args;

    let args = Args::try_parse_from(args)?;
    args.run().await?;

    // Check that bruno.json was created
    let bruno_json = output_dir.path().join("bruno.json");
    assert!(bruno_json.exists(), "bruno.json should be created");

    // Check that environment file was created
    let env_file = output_dir.path().join("environments/default.bru");
    assert!(env_file.exists(), "environment file should be created");

    // Check that method files were created
    for (index, method) in expected_methods.iter().enumerate() {
        let method_file = output_dir
            .path()
            .join(format!("{:03}_{}.bru", index + 1, method));
        assert!(
            method_file.exists(),
            "method file {} should be created",
            method
        );

        // Verify the file has valid content
        let content = fs::read_to_string(&method_file).await?;
        assert!(
            content.contains("meta {"),
            "method file should have meta section"
        );
        assert!(
            content.contains("post {"),
            "method file should have post section"
        );
        assert!(
            content.contains("body:json {"),
            "method file should have body section"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_rejects_path_traversal_method_name() {
    use clap::Parser;
    use openrpc_to_bruno::{cli::Args, error::ToolError};

    let temp = tempdir().unwrap();
    let input_path = temp.path().join("openrpc.json");
    let output_dir = temp.path().join("out");
    let escaped = temp.path().join("evil.bru");

    let spec = serde_json::json!({
        "openrpc": "1.3.2",
        "info": {
            "title": "Unsafe API",
            "version": "1.0.0"
        },
        "methods": [{
            "name": "../evil",
            "params": [],
            "result": {
                "name": "result",
                "schema": { "type": "string" }
            }
        }]
    });
    fs::write(&input_path, serde_json::to_vec(&spec).unwrap())
        .await
        .unwrap();

    let args = Args::try_parse_from(vec![
        "openrpc-to-bruno",
        "--input",
        input_path.to_str().unwrap(),
        "--output",
        output_dir.to_str().unwrap(),
        "--force",
    ])
    .unwrap();

    let err = args.run().await.unwrap_err();
    assert!(matches!(err, ToolError::UnsafeMethodName(_)));
    assert!(!escaped.exists());
}

#[tokio::test]
async fn test_sanitizes_safe_method_filename() {
    use clap::Parser;
    use openrpc_to_bruno::cli::Args;

    let temp = tempdir().unwrap();
    let input_path = temp.path().join("openrpc.json");
    let output_dir = temp.path().join("out");

    let spec = serde_json::json!({
        "openrpc": "1.3.2",
        "info": {
            "title": "Safe API",
            "version": "1.0.0"
        },
        "methods": [{
            "name": "system status",
            "params": [],
            "result": {
                "name": "result",
                "schema": { "type": "string" }
            }
        }]
    });
    fs::write(&input_path, serde_json::to_vec(&spec).unwrap())
        .await
        .unwrap();

    let args = Args::try_parse_from(vec![
        "openrpc-to-bruno",
        "--input",
        input_path.to_str().unwrap(),
        "--output",
        output_dir.to_str().unwrap(),
        "--force",
    ])
    .unwrap();

    args.run().await.unwrap();
    assert!(output_dir.join("001_system_status.bru").exists());
}

#[tokio::test]
async fn test_simple_conversion() {
    test_conversion("simple-api-basic.json", &["hello"])
        .await
        .expect("Simple conversion should work");
}

#[tokio::test]
async fn test_collection_metadata() {
    let input_path = Path::new("tests/fixtures/simple-api-basic.json");
    let output_dir = tempdir().unwrap();

    use clap::Parser;
    use openrpc_to_bruno::cli::Args;

    let args = Args::try_parse_from(vec![
        "openrpc-to-bruno",
        "--input",
        input_path.to_str().unwrap(),
        "--output",
        output_dir.path().to_str().unwrap(),
        "--name",
        "Custom Collection Name",
        "--force",
    ])
    .unwrap();

    args.run().await.unwrap();

    // Check bruno.json content
    let bruno_json = output_dir.path().join("bruno.json");
    let content = fs::read_to_string(bruno_json).await.unwrap();
    assert!(
        content.contains("Custom Collection Name"),
        "Should use custom collection name"
    );
}

#[tokio::test]
async fn test_environment_variables() {
    let input_path = Path::new("tests/fixtures/simple-api-basic.json");
    let output_dir = tempdir().unwrap();

    use clap::Parser;
    use openrpc_to_bruno::cli::Args;

    let args = Args::try_parse_from(vec![
        "openrpc-to-bruno",
        "--input",
        input_path.to_str().unwrap(),
        "--output",
        output_dir.path().to_str().unwrap(),
        "--base-url",
        "https://api.example.com",
        "--force",
    ])
    .unwrap();

    args.run().await.unwrap();

    // Check environment file content
    let env_file = output_dir.path().join("environments/default.bru");
    let content = fs::read_to_string(env_file).await.unwrap();
    assert!(
        content.contains("https://api.example.com"),
        "Should use custom base URL"
    );
}
