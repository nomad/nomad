use std::sync::{Arc, LazyLock};

use jsonwebtoken::Algorithm;

use crate::Claims;
use crate::audience::{Audience, Client, CollabServer};

static AUTH_SERVER_JWT_SIGNING_PUBLIC_KEY: LazyLock<
    jsonwebtoken::DecodingKey,
> = LazyLock::new(|| {
    #[cfg(not(feature = "tests"))]
    let contents = include_bytes!("../auth_server_jwt_signing_public_key.pem");

    #[cfg(feature = "tests")]
    let contents =
        include_bytes!("../tests/auth_server_jwt_signing_public_key.pem");

    jsonwebtoken::DecodingKey::from_ec_pem(contents)
        .expect("public key is valid")
});

/// The JWT returned by Nomad's auth server, along with its parsed [`Claims`].
#[derive(Clone)]
pub struct JsonWebToken {
    contents: Arc<str>,
    claims: Claims,
}

impl JsonWebToken {
    /// Returns the token's contents.
    pub fn as_str(&self) -> &str {
        &self.contents
    }

    /// Returns the token's claims.
    pub fn claims(&self) -> &Claims {
        &self.claims
    }

    /// Parses the string as a `JsonWebToken`, validating against the given
    /// audience type.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<Aud: Audience>(
        str: &str,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        let mut validation = jsonwebtoken::Validation::new(Algorithm::ES256);
        Aud::set_audience(&mut validation);
        validation.set_issuer(&[crate::JWT_ISSUER]);

        let token_data = jsonwebtoken::decode::<Claims>(
            str,
            &AUTH_SERVER_JWT_SIGNING_PUBLIC_KEY,
            &validation,
        )?;

        Ok(Self { contents: str.into(), claims: token_data.claims })
    }

    /// Calls [`from_str`](Self::from_str) with the [`Client`] audience.
    pub fn from_str_on_client(
        str: &str,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        Self::from_str::<Client>(str)
    }

    /// Calls [`from_str`](Self::from_str) with the [`CollabServer`] audience.
    pub fn from_str_on_collab_server(
        str: &str,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        Self::from_str::<CollabServer>(str)
    }

    /// Creates a mock `JsonWebToken` for the given user.
    #[cfg(feature = "tests")]
    pub fn mock(username: peer_handle::PeerHandle) -> Self {
        let claims = Claims {
            audience: "tests".into(),
            expires_at: u64::MAX,
            issued_at: 0,
            issuer: env!("CARGO_PKG_NAME").into(),
            subject: crate::Subject::GitHubUserId(1),
            email: format!("{}@example.com", username.as_str())
                .parse()
                .expect("valid email address"),
            name: Some("Test User".into()),
            username,
        };

        let signing_key = jsonwebtoken::EncodingKey::from_ec_pem(
            include_bytes!("../tests/auth_server_jwt_signing_private_key.pem"),
        )
        .expect("private key is valid");

        let contents = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(Algorithm::ES256),
            &claims,
            &signing_key,
        )
        .expect("couldn't encode mock JWT");

        Self { contents: contents.into(), claims }
    }
}

#[cfg(feature = "tests")]
impl From<JsonWebToken> for peer_handle::PeerHandle {
    fn from(jwt: JsonWebToken) -> Self {
        jwt.claims().username.clone()
    }
}
