#[test]
fn test_generated_explorer_does_not_store_jwt_in_local_storage() {
    let template = include_str!("../src/jsonrpc_explorer_template.html");
    assert!(!template.contains("localStorage.getItem('jwt-token')"));
    assert!(!template.contains("localStorage.setItem('jwt-token'"));
    assert!(!template.contains("localStorage.removeItem('jwt-token'"));
    assert!(template.contains("sessionStorage.setItem('jwt-token'"));
}
