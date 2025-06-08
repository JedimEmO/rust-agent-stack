use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bruno collection metadata file (bruno.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct BrunoCollection {
    pub version: String,
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: String,
    pub ignore: Vec<String>,
}

impl Default for BrunoCollection {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            collection_type: "collection".to_string(),
            ignore: vec!["node_modules".to_string(), ".git".to_string()],
            name: "generated-collection".to_string(),
        }
    }
}

/// Bruno environment variables file (.bru)
#[derive(Debug)]
pub struct BrunoEnvironment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

impl BrunoEnvironment {
    pub fn new(name: String) -> Self {
        Self {
            name,
            variables: HashMap::new(),
        }
    }

    pub fn add_variable<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.variables.insert(key.into(), value.into());
    }

    pub fn to_bru_format(&self) -> String {
        let mut content = String::new();

        if !self.variables.is_empty() {
            content.push_str("vars {\n");
            for (key, value) in &self.variables {
                content.push_str(&format!("  {}: {}\n", key, value));
            }
            content.push_str("}\n");
        }

        content
    }
}

/// Bruno request file (.bru)
#[derive(Debug)]
pub struct BrunoRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<BrunoRequestBody>,
    pub auth: Option<BrunoAuth>,
    pub sequence: Option<u32>,
}

#[derive(Debug)]
pub enum BrunoRequestBody {
    Json(String),
    Text(String),
}

#[derive(Debug)]
pub enum BrunoAuth {
    Bearer { token: String },
    Basic { username: String, password: String },
    None,
}

impl BrunoRequest {
    pub fn new<S: Into<String>>(name: S, method: S, url: S) -> Self {
        Self {
            name: name.into(),
            method: method.into(),
            url: url.into(),
            headers: HashMap::new(),
            body: None,
            auth: None,
            sequence: None,
        }
    }

    pub fn with_sequence(mut self, seq: u32) -> Self {
        self.sequence = Some(seq);
        self
    }

    pub fn with_json_body<S: Into<String>>(mut self, body: S) -> Self {
        self.body = Some(BrunoRequestBody::Json(body.into()));
        self
    }

    pub fn with_auth(mut self, auth: BrunoAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn add_header<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.headers.insert(key.into(), value.into());
    }

    pub fn to_bru_format(&self) -> String {
        let mut content = String::new();

        // Meta section
        content.push_str("meta {\n");
        content.push_str(&format!("  name: {}\n", self.name));
        content.push_str("  type: http\n");
        if let Some(seq) = self.sequence {
            content.push_str(&format!("  seq: {}\n", seq));
        }
        content.push_str("}\n\n");

        // Request section
        content.push_str(&format!("{} {{\n", self.method.to_lowercase()));
        content.push_str(&format!("  url: {}\n", self.url));

        if self.body.is_some() {
            content.push_str("  body: json\n");
        }

        if self.auth.is_some() {
            match &self.auth {
                Some(BrunoAuth::Bearer { .. }) => content.push_str("  auth: bearer\n"),
                Some(BrunoAuth::Basic { .. }) => content.push_str("  auth: basic\n"),
                _ => {}
            }
        }

        content.push_str("}\n");

        // Headers section
        if !self.headers.is_empty() {
            content.push_str("\nheaders {\n");
            for (key, value) in &self.headers {
                content.push_str(&format!("  {}: {}\n", key, value));
            }
            content.push_str("}\n");
        }

        // Auth section
        if let Some(auth) = &self.auth {
            match auth {
                BrunoAuth::Bearer { token } => {
                    content.push_str("\nauth:bearer {\n");
                    content.push_str(&format!("  token: {}\n", token));
                    content.push_str("}\n");
                }
                BrunoAuth::Basic { username, password } => {
                    content.push_str("\nauth:basic {\n");
                    content.push_str(&format!("  username: {}\n", username));
                    content.push_str(&format!("  password: {}\n", password));
                    content.push_str("}\n");
                }
                _ => {}
            }
        }

        // Body section
        if let Some(body) = &self.body {
            match body {
                BrunoRequestBody::Json(json) => {
                    content.push_str("\nbody:json {\n");
                    // Indent each line of the JSON with 2 spaces
                    for line in json.lines() {
                        content.push_str(&format!("  {}\n", line));
                    }
                    content.push_str("}\n");
                }
                BrunoRequestBody::Text(text) => {
                    content.push_str("\nbody:text {\n");
                    content.push_str(&format!("  {}\n", text));
                    content.push_str("}\n");
                }
            }
        }

        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bruno_collection_default() {
        let collection = BrunoCollection::default();
        assert_eq!(collection.version, "1");
        assert_eq!(collection.collection_type, "collection");
        assert_eq!(collection.name, "generated-collection");
        assert_eq!(collection.ignore, vec!["node_modules", ".git"]);
    }

    #[test]
    fn test_bruno_environment_to_bru_format() {
        let mut env = BrunoEnvironment::new("test".to_string());
        env.add_variable("host", "localhost:3000");
        env.add_variable("api_path", "/api/v1/rpc");

        let output = env.to_bru_format();
        assert!(output.contains("vars {"));
        assert!(output.contains("host: localhost:3000"));
        assert!(output.contains("api_path: /api/v1/rpc"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_bruno_request_basic() {
        let request =
            BrunoRequest::new("test_method", "post", "http://localhost:3000/api").with_sequence(1);

        let output = request.to_bru_format();
        assert!(output.contains("meta {"));
        assert!(output.contains("name: test_method"));
        assert!(output.contains("type: http"));
        assert!(output.contains("seq: 1"));
        assert!(output.contains("post {"));
        assert!(output.contains("url: http://localhost:3000/api"));
    }

    #[test]
    fn test_bruno_request_with_json_body() {
        let request = BrunoRequest::new("test", "post", "http://localhost:3000")
            .with_json_body(r#"{"test": "value"}"#);

        let output = request.to_bru_format();
        assert!(output.contains("body: json"));
        assert!(output.contains("body:json {"));
        assert!(output.contains(r#"{"test": "value"}"#));
    }

    #[test]
    fn test_bruno_request_with_bearer_auth() {
        let request = BrunoRequest::new("test", "post", "http://localhost:3000").with_auth(
            BrunoAuth::Bearer {
                token: "{{auth_token}}".to_string(),
            },
        );

        let output = request.to_bru_format();
        assert!(output.contains("auth: bearer"));
        assert!(output.contains("auth:bearer {"));
        assert!(output.contains("token: {{auth_token}}"));
    }

    #[test]
    fn test_bruno_request_with_headers() {
        let mut request = BrunoRequest::new("test", "post", "http://localhost:3000");
        request.add_header("Content-Type", "application/json");
        request.add_header("Accept", "application/json");

        let output = request.to_bru_format();
        assert!(output.contains("headers {"));
        assert!(output.contains("Content-Type: application/json"));
        assert!(output.contains("Accept: application/json"));
    }

    #[test]
    fn test_bruno_request_json_indentation() {
        // Test that multi-line JSON is properly indented
        let json_body = r#"{
  "jsonrpc": "2.0",
  "method": "test_method",
  "params": {
    "name": "value",
    "number": 42
  },
  "id": 1
}"#;

        let request =
            BrunoRequest::new("test", "post", "http://localhost:3000").with_json_body(json_body);

        let output = request.to_bru_format();

        // Each line of JSON should be indented with 2 spaces inside body:json { }
        assert!(output.contains("body:json {\n  {\n    \"jsonrpc\": \"2.0\","));
        assert!(output.contains("    \"method\": \"test_method\","));
        assert!(output.contains("    \"params\": {"));
        assert!(output.contains("      \"name\": \"value\","));
        assert!(output.contains("      \"number\": 42"));
        assert!(output.contains("    },"));
        assert!(output.contains("    \"id\": 1"));
        assert!(output.contains("  }\n}"));
    }
}
