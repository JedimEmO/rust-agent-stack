pub struct VerifiedClientProperties {
    pub client_id: String,
}

pub trait IdentifyProvider {
    fn provider_id(&self) -> String;
    async fn auth(&self, auth_payload: serde_json::Value) -> AuthResult<VerifiedClientProperties>;
}


// for instance, an oauth2 impl of this provider:

#[derive(Serialize, Deserialize)]
struct OAuth2Params {
    auth_token: String
}

struct OAuth2Provider {}

impl IdentifyProvider for OAuth2Params {
    ...
}

// We need an internal session service to translate authenticated identities into internal user sessions with our own JWT
// I'm thinking session service could eventually use an internal event/messaging system to distribute active session information
// across multiple deployment units, but not required in the first iteration.
pub struct SessionService {}

impl SessionService {
    pub fn begin_session(&self, identity: VerifiedClientProperties) {}
    pub fn renew_session(&self, identity: VerifiedClientProperties, session: JWT) {}
    pub fn end_session(&self, session: JWT) {}
}