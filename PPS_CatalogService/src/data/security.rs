use crate::{
    application::{Permission, Principal, RequestAuthorizer, RequestIdentity},
    AppError,
};
use async_trait::async_trait;

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
