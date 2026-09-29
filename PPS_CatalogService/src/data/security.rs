use crate::{
    application::{Permission, Principal, RequestAuthorizer, RequestIdentity},
    AppError,
};
use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::RwLock, time::sleep};

const CATALOG_ADMIN_SCOPE: &str = "pps-api/catalog.admin";
const CATALOG_ADMIN_GROUP: &str = "pps-admins";
const JWKS_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
pub struct CognitoAuthorizer {
    issuer: String,
    client_id: String,
    jwks_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<KeyCache>>,
}

#[derive(Default)]
struct KeyCache {
    keys: HashMap<String, DecodingKey>,
    refreshed_at: Option<Instant>,
}

#[derive(Debug, Deserialize)]
struct CognitoClaims {
    sub: String,
    iss: String,
    exp: usize,
    client_id: String,
    token_use: String,
    #[serde(default)]
    scope: String,
    #[serde(rename = "cognito:groups", default)]
    groups: Vec<String>,
}

impl CognitoAuthorizer {
    pub fn new(issuer: String, client_id: String, jwks_url: String) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|error| {
                AppError::Configuration(format!("create Cognito HTTP client: {error}"))
            })?;
        Ok(Self {
            issuer: issuer.trim_end_matches('/').into(),
            client_id,
            jwks_url,
            client,
            cache: Arc::new(RwLock::new(KeyCache::default())),
        })
    }

    async fn key(&self, kid: &str) -> Result<DecodingKey, AppError> {
        {
            let cache = self.cache.read().await;
            if cache.refreshed_at.is_some_and(|at| at.elapsed() < JWKS_TTL) {
                if let Some(key) = cache.keys.get(kid) {
                    return Ok(key.clone());
                }
            }
        }
        self.refresh(kid).await?;
        self.cache
            .read()
            .await
            .keys
            .get(kid)
            .cloned()
            .ok_or(AppError::Unauthorized)
    }

    async fn refresh(&self, wanted_kid: &str) -> Result<(), AppError> {
        let mut cache = self.cache.write().await;
        if cache.refreshed_at.is_some_and(|at| at.elapsed() < JWKS_TTL)
            && cache.keys.contains_key(wanted_kid)
        {
            return Ok(());
        }
        let mut last_error = String::new();
        for (attempt, delay) in [100_u64, 250, 500].into_iter().enumerate() {
            match self.client.get(&self.jwks_url).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.json::<JwkSet>().await {
                        Ok(set) => {
                            let keys = set
                                .keys
                                .into_iter()
                                .filter_map(|jwk| {
                                    let kid = jwk.common.key_id.clone()?;
                                    DecodingKey::from_jwk(&jwk).ok().map(|key| (kid, key))
                                })
                                .collect::<HashMap<_, _>>();
                            if !keys.is_empty() {
                                cache.keys = keys;
                                cache.refreshed_at = Some(Instant::now());
                                return Ok(());
                            }
                            last_error = "Cognito JWKS contained no usable keys".into();
                        }
                        Err(error) => last_error = format!("decode Cognito JWKS: {error}"),
                    }
                }
                Ok(response) => last_error = format!("Cognito JWKS returned {}", response.status()),
                Err(error) => last_error = format!("fetch Cognito JWKS: {error}"),
            }
            if attempt < 2 {
                sleep(Duration::from_millis(delay)).await;
            }
        }
        Err(AppError::ServiceUnavailable(last_error))
    }
}

#[async_trait]
impl RequestAuthorizer for CognitoAuthorizer {
    async fn authorize(
        &self,
        identity: RequestIdentity,
        _: Permission,
    ) -> Result<Principal, AppError> {
        let token = identity.bearer_token.ok_or(AppError::Unauthorized)?;
        let kid = decode_header(&token)
            .map_err(|_| AppError::Unauthorized)?
            .kid
            .ok_or(AppError::Unauthorized)?;
        let key = self.key(&kid).await?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation.validate_aud = false;
        let claims = decode::<CognitoClaims>(&token, &key, &validation)
            .map_err(|_| AppError::Unauthorized)?
            .claims;
        if claims.iss != self.issuer
            || claims.client_id != self.client_id
            || claims.token_use != "access"
            || claims.exp == 0
        {
            return Err(AppError::Unauthorized);
        }
        if !claims
            .scope
            .split_whitespace()
            .any(|scope| scope == CATALOG_ADMIN_SCOPE)
        {
            return Err(AppError::Forbidden);
        }
        if !claims
            .groups
            .iter()
            .any(|group| group == CATALOG_ADMIN_GROUP)
        {
            return Err(AppError::Forbidden);
        }
        Ok(Principal {
            subject: claims.sub,
        })
    }
}

pub struct DevelopmentAuthorizer {
    expected_token: String,
}

impl DevelopmentAuthorizer {
    pub fn new(expected_token: String) -> Result<Self, AppError> {
        if expected_token.len() < 24 {
            return Err(AppError::Configuration(
                "ADMIN_DEV_TOKEN must contain at least 24 characters".into(),
            ));
        }
        Ok(Self { expected_token })
    }
}

#[async_trait]
impl RequestAuthorizer for DevelopmentAuthorizer {
    async fn authorize(
        &self,
        identity: RequestIdentity,
        _: Permission,
    ) -> Result<Principal, AppError> {
        let supplied = identity.development_token.ok_or(AppError::Unauthorized)?;
        if constant_time_equal(supplied.as_bytes(), self.expected_token.as_bytes()) {
            Ok(Principal {
                subject: "local-admin".into(),
            })
        } else {
            Err(AppError::Forbidden)
        }
    }
}

pub struct DisabledAuthorizer;

#[async_trait]
impl RequestAuthorizer for DisabledAuthorizer {
    async fn authorize(&self, _: RequestIdentity, _: Permission) -> Result<Principal, AppError> {
        Err(AppError::Forbidden)
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn development_authorizer_rejects_wrong_token() {
        let authorizer =
            DevelopmentAuthorizer::new("a-very-long-development-token".into()).unwrap();
        let result = authorizer
            .authorize(
                RequestIdentity {
                    development_token: Some("wrong".into()),
                    bearer_token: None,
                },
                Permission::CatalogAdmin,
            )
            .await;
        assert!(matches!(result, Err(AppError::Forbidden)));
    }
}
