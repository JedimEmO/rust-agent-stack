//! End-to-end test: generated reqwest client → axum router → trait impl →
//! response → client. Covers GET, POST with body, path params, query params,
//! and auth-related rejection paths.

use ras_auth_core::AuthenticatedUser;
use ras_rest_core::{RestError, RestResponse, RestResult};
use ras_rest_macro::rest_service;
use ras_test_helpers::{MockAuthProvider, spawn_http};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct Item {
    id: u32,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct CreateItem {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ItemsResponse {
    items: Vec<Item>,
}

rest_service!({
    service_name: Demo,
    base_path: "/api",
    openapi: false,
    serve_docs: false,
    endpoints: [
        GET UNAUTHORIZED items() -> ItemsResponse,
        GET WITH_PERMISSIONS(["user"]) items/{id: u32}() -> Item,
        POST WITH_PERMISSIONS(["admin"]) items(CreateItem) -> Item,
        GET UNAUTHORIZED search ? q: String & limit: Option<u32> & exact: bool () -> ItemsResponse,
        POST WITH_PERMISSIONS(["admin"]) items/batch ? notify: bool (CreateItem) -> Item,
        GET WITH_PERMISSIONS(["user"]) items/{id: u32}/related ? tag: Option<String> () -> ItemsResponse,
    ]
});

struct DemoImpl;

#[async_trait::async_trait]
impl DemoTrait for DemoImpl {
    async fn get_items(&self) -> RestResult<ItemsResponse> {
        Ok(RestResponse::ok(ItemsResponse {
            items: vec![Item {
                id: 1,
                name: "alpha".into(),
            }],
        }))
    }

    async fn get_items_by_id(&self, _user: &AuthenticatedUser, id: u32) -> RestResult<Item> {
        if id == 404 {
            Err(RestError::not_found("missing"))
        } else {
            Ok(RestResponse::ok(Item {
                id,
                name: format!("item-{id}"),
            }))
        }
    }

    async fn post_items(&self, user: &AuthenticatedUser, body: CreateItem) -> RestResult<Item> {
        // Use the user_id length so we can verify the user actually arrived.
        Ok(RestResponse::created(Item {
            id: user.user_id.len() as u32,
            name: body.name,
        }))
    }

    async fn get_search(
        &self,
        q: String,
        limit: Option<u32>,
        exact: bool,
    ) -> RestResult<ItemsResponse> {
        let n = limit.unwrap_or(2);
        let prefix = if exact { "exact" } else { "fuzzy" };
        let items = (0..n)
            .map(|i| Item {
                id: i,
                name: format!("{prefix}:{q}-{i}"),
            })
            .collect();
        Ok(RestResponse::ok(ItemsResponse { items }))
    }

    async fn post_items_batch(
        &self,
        _user: &AuthenticatedUser,
        notify: bool,
        body: CreateItem,
    ) -> RestResult<Item> {
        // Encode the bool query param into the response so we can assert on it.
        let suffix = if notify { "(notified)" } else { "(silent)" };
        Ok(RestResponse::created(Item {
            id: 0,
            name: format!("{}{suffix}", body.name),
        }))
    }

    async fn get_items_by_id_related(
        &self,
        _user: &AuthenticatedUser,
        id: u32,
        tag: Option<String>,
    ) -> RestResult<ItemsResponse> {
        let label = tag.unwrap_or_else(|| "none".into());
        Ok(RestResponse::ok(ItemsResponse {
            items: vec![Item {
                id,
                name: format!("related/{label}"),
            }],
        }))
    }
}

fn router() -> axum::Router {
    DemoBuilder::new(DemoImpl)
        .auth_provider(MockAuthProvider::default())
        .build()
}

fn client(base: &str) -> DemoClient {
    DemoClient::builder(base).build().expect("client build")
}

#[tokio::test]
async fn unauth_get_round_trips() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let resp = client(&base).get_items().await.expect("get_items ok");
    assert_eq!(resp.items.len(), 1);
    assert_eq!(resp.items[0].name, "alpha");
}

#[tokio::test]
async fn auth_get_with_path_param_succeeds_with_user_token() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("user-token".to_string()));

    let item = c.get_items_by_id(7).await.expect("get_items_by_id ok");
    assert_eq!(item.id, 7);
    assert_eq!(item.name, "item-7");
}

#[tokio::test]
async fn auth_get_rejected_without_token() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    // No bearer token set on client.
    let err = client(&base)
        .get_items_by_id(1)
        .await
        .expect_err("must be rejected");
    let s = err.to_string();
    assert!(s.contains("401") || s.contains("Unauthorized"), "got: {s}");
}

#[tokio::test]
async fn auth_post_rejected_with_insufficient_perms() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("user-token".to_string())); // not admin

    let err = c
        .post_items(CreateItem {
            name: "x".to_string(),
        })
        .await
        .expect_err("user-token can't POST items");
    let s = err.to_string();
    assert!(s.contains("403") || s.contains("Forbidden"), "got: {s}");
}

#[tokio::test]
async fn auth_post_with_admin_succeeds_and_user_id_propagates() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("admin-token".to_string()));

    let item = c
        .post_items(CreateItem { name: "foo".into() })
        .await
        .expect("post_items ok");
    assert_eq!(item.name, "foo");
    // admin-1 is 7 chars long.
    assert_eq!(item.id, 7);
}

#[tokio::test]
async fn query_params_required_and_optional_serialize_correctly() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();

    // Optional `limit` provided, required `q` and `exact` set.
    let resp = client(&base)
        .get_search("hi".to_string(), Some(3), true)
        .await
        .expect("search ok");
    assert_eq!(resp.items.len(), 3);
    assert_eq!(resp.items[0].name, "exact:hi-0");
    assert_eq!(resp.items[2].name, "exact:hi-2");

    // Optional `limit` omitted (None) → handler default of 2 applies, and the
    // bool flips the prefix.
    let resp = client(&base)
        .get_search("zz".to_string(), None, false)
        .await
        .expect("search ok");
    assert_eq!(resp.items.len(), 2);
    assert_eq!(resp.items[0].name, "fuzzy:zz-0");
}

#[tokio::test]
async fn query_params_with_body_and_auth() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("admin-token".to_string()));

    let item = c
        .post_items_batch(
            true,
            CreateItem {
                name: "alpha".into(),
            },
        )
        .await
        .expect("post_items_batch ok");
    assert_eq!(item.name, "alpha(notified)");

    let item = c
        .post_items_batch(
            false,
            CreateItem {
                name: "beta".into(),
            },
        )
        .await
        .expect("post_items_batch ok");
    assert_eq!(item.name, "beta(silent)");
}

#[tokio::test]
async fn query_params_with_path_param() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("user-token".to_string()));

    let resp = c
        .get_items_by_id_related(42, Some("featured".into()))
        .await
        .expect("related with tag");
    assert_eq!(resp.items[0].id, 42);
    assert_eq!(resp.items[0].name, "related/featured");

    let resp = c
        .get_items_by_id_related(42, None)
        .await
        .expect("related without tag");
    assert_eq!(resp.items[0].name, "related/none");
}

#[tokio::test]
async fn handler_error_surfaces_to_client() {
    let server = spawn_http(router());
    let base = server.server_address().unwrap().to_string();
    let mut c = client(&base);
    c.set_bearer_token(Some("user-token".to_string()));

    let err = c
        .get_items_by_id(404)
        .await
        .expect_err("404 sentinel must error");
    assert!(err.to_string().contains("404"), "got: {err}");
}
