use crate::common::entities::app_errors::CoreError;
use crate::common::generate_uuid_v7;
use crate::user::entities::User;
use crate::user_passkey::entities::UserPasskeyCredential;
use crate::user_passkey::ports::{PasskeyChallengeStore, UserPasskeyRepository};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, Credential, CredentialID, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Webauthn, WebauthnBuilder,
};

const PASSKEY_CHALLENGE_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyLoginState {
    pub realm_id: String,
    pub client_id: String,
    pub client_ip: String,
    #[serde(default)]
    pub oauth_client_id: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistrationChallengeState {
    realm_id: String,
    user_id: Uuid,
    nickname: Option<String>,
    state: PasskeyRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthenticationChallengeState {
    login_state: PasskeyLoginState,
    state: PasskeyAuthentication,
}

#[derive(Debug, thiserror::Error)]
pub enum PasskeyError {
    #[error("Passkey 功能未启用")]
    Disabled,
    #[error("Passkey 验证失败")]
    VerificationFailed,
    #[error("未找到可用的 Passkey")]
    NotFound,
    #[error("challenge 已过期，请重新发起")]
    ChallengeExpired,
    #[error("浏览器或设备不支持 Passkey")]
    Unsupported,
    #[error(transparent)]
    Repo(#[from] CoreError),
}

pub struct UserPasskeyService<R, S>
where
    R: UserPasskeyRepository,
    S: PasskeyChallengeStore,
{
    webauthn: Webauthn,
    repo: Arc<R>,
    challenge_store: Arc<S>,
}

impl<R, S> UserPasskeyService<R, S>
where
    R: UserPasskeyRepository,
    S: PasskeyChallengeStore,
{
    pub fn new(
        rp_id: &str,
        rp_origin: &str,
        repo: Arc<R>,
        challenge_store: Arc<S>,
    ) -> Result<Self, PasskeyError> {
        let rp_origin = Url::parse(rp_origin).map_err(|_| PasskeyError::VerificationFailed)?;
        let webauthn = WebauthnBuilder::new(rp_id, &rp_origin)
            .map_err(|_| PasskeyError::VerificationFailed)?
            .build()
            .map_err(|_| PasskeyError::VerificationFailed)?;

        Ok(Self {
            webauthn,
            repo,
            challenge_store,
        })
    }

    pub async fn begin_registration(
        &self,
        realm_id: &str,
        user: &User,
        exclude: &[Vec<u8>],
    ) -> Result<(CreationChallengeResponse, String), PasskeyError> {
        let exclude_credentials = exclude
            .iter()
            .cloned()
            .map(CredentialID::from)
            .collect::<Vec<_>>();
        let display_name = user.nickname.as_deref().unwrap_or(&user.email);
        let (challenge, state) = self
            .webauthn
            .start_passkey_registration(
                user.id,
                &user.email,
                display_name,
                Some(exclude_credentials),
            )
            .map_err(|_| PasskeyError::VerificationFailed)?;

        let reg_token = generate_token();
        let payload = RegistrationChallengeState {
            realm_id: realm_id.to_string(),
            user_id: user.id,
            nickname: None,
            state,
        };
        self.store_challenge(&reg_key(&reg_token), &payload).await?;

        Ok((challenge, reg_token))
    }

    pub async fn finish_registration(
        &self,
        reg_token: &str,
        resp_json: &Value,
        nickname: Option<&str>,
    ) -> Result<UserPasskeyCredential, PasskeyError> {
        let key = reg_key(reg_token);
        let payload = self
            .load_challenge::<RegistrationChallengeState>(&key)
            .await?;
        self.challenge_store.delete(&key).await?;

        let response: RegisterPublicKeyCredential = serde_json::from_value(resp_json.clone())
            .map_err(|_| PasskeyError::VerificationFailed)?;
        let passkey = self
            .webauthn
            .finish_passkey_registration(&response, &payload.state)
            .map_err(|_| PasskeyError::VerificationFailed)?;
        let passkey_payload =
            serde_json::to_vec(&passkey).map_err(|_| PasskeyError::VerificationFailed)?;
        let credential = Credential::from(passkey);

        let now = Utc::now();
        let credential = UserPasskeyCredential {
            id: generate_uuid_v7(),
            user_id: payload.user_id,
            realm_id: payload.realm_id,
            credential_id: credential.cred_id.to_vec(),
            credential_public_key: passkey_payload,
            counter: u64::from(credential.counter),
            transports: credential
                .transports
                .map(|transports| transports.iter().map(|t| format!("{t:?}")).collect())
                .unwrap_or_default(),
            aaguid: None,
            backup_eligible: credential.backup_eligible,
            backup_state: credential.backup_state,
            user_verified: credential.user_verified,
            nickname: nickname.map(str::to_string).or(payload.nickname),
            last_used_at: None,
            created_at: now,
            updated_at: now,
        };

        self.repo
            .insert(credential)
            .await
            .map_err(PasskeyError::Repo)
    }

    pub async fn begin_login_first_factor(
        &self,
        _realm_id: &str,
        state: PasskeyLoginState,
    ) -> Result<(RequestChallengeResponse, String), PasskeyError> {
        let passkeys = Vec::new();
        let (challenge, auth_state) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|_| PasskeyError::VerificationFailed)?;

        let auth_token = generate_token();
        let payload = AuthenticationChallengeState {
            login_state: state,
            state: auth_state,
        };
        self.store_challenge(&auth_key(&auth_token), &payload)
            .await?;

        Ok((challenge, auth_token))
    }

    pub async fn finish_login_first_factor(
        &self,
        auth_token: &str,
        resp_json: &Value,
    ) -> Result<(Uuid, UserPasskeyCredential, PasskeyLoginState), PasskeyError> {
        let (credential, login_state) = self
            .finish_authentication(&auth_key(auth_token), resp_json)
            .await?;
        let user_id = credential.user_id;

        Ok((user_id, credential, login_state))
    }

    pub async fn begin_second_factor(
        &self,
        temp_session: &PasskeyLoginState,
        user_id: Uuid,
    ) -> Result<(RequestChallengeResponse, String), PasskeyError> {
        let credentials = self
            .repo
            .list_by_user(&temp_session.realm_id, user_id)
            .await
            .map_err(PasskeyError::Repo)?;
        if credentials.is_empty() {
            return Err(PasskeyError::NotFound);
        }

        let passkeys = credentials_to_passkeys(credentials)?;
        let (challenge, auth_state) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|_| PasskeyError::VerificationFailed)?;

        let token = generate_token();
        let payload = AuthenticationChallengeState {
            login_state: temp_session.clone(),
            state: auth_state,
        };
        self.store_challenge(&two_factor_key(&token), &payload)
            .await?;

        Ok((challenge, token))
    }

    pub async fn finish_second_factor(
        &self,
        token: &str,
        resp_json: &Value,
    ) -> Result<UserPasskeyCredential, PasskeyError> {
        let (credential, _) = self
            .finish_authentication(&two_factor_key(token), resp_json)
            .await?;
        Ok(credential)
    }

    async fn finish_authentication(
        &self,
        key: &str,
        resp_json: &Value,
    ) -> Result<(UserPasskeyCredential, PasskeyLoginState), PasskeyError> {
        let payload = self
            .load_challenge::<AuthenticationChallengeState>(key)
            .await?;
        self.challenge_store.delete(key).await?;

        let response: PublicKeyCredential = serde_json::from_value(resp_json.clone())
            .map_err(|_| PasskeyError::VerificationFailed)?;
        let auth_result = self
            .webauthn
            .finish_passkey_authentication(&response, &payload.state)
            .map_err(|_| PasskeyError::VerificationFailed)?;
        let credential_id = auth_result.cred_id().to_vec();
        let credential = self
            .repo
            .find_by_credential_id(&payload.login_state.realm_id, &credential_id)
            .await
            .map_err(PasskeyError::Repo)?
            .ok_or(PasskeyError::VerificationFailed)?;
        let new_counter = u64::from(auth_result.counter());
        if new_counter <= credential.counter {
            return Err(PasskeyError::VerificationFailed);
        }

        self.repo
            .update_counter_and_used(credential.id, new_counter, Utc::now())
            .await
            .map_err(PasskeyError::Repo)?;

        let mut updated = credential;
        updated.counter = new_counter;
        updated.last_used_at = Some(Utc::now());

        Ok((updated, payload.login_state))
    }

    async fn store_challenge<T: Serialize>(
        &self,
        key: &str,
        payload: &T,
    ) -> Result<(), PasskeyError> {
        let serialized =
            serde_json::to_vec(payload).map_err(|_| PasskeyError::VerificationFailed)?;
        self.challenge_store
            .store(key, &serialized, PASSKEY_CHALLENGE_TTL_SECONDS)
            .await?;
        Ok(())
    }

    async fn load_challenge<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<T, PasskeyError> {
        let payload = self
            .challenge_store
            .load(key)
            .await?
            .ok_or(PasskeyError::ChallengeExpired)?;
        serde_json::from_slice(&payload).map_err(|_| PasskeyError::VerificationFailed)
    }
}

fn credentials_to_passkeys(
    credentials: Vec<UserPasskeyCredential>,
) -> Result<Vec<Passkey>, PasskeyError> {
    credentials
        .into_iter()
        .map(|credential| {
            serde_json::from_slice(&credential.credential_public_key)
                .map_err(|_| PasskeyError::VerificationFailed)
        })
        .collect()
}

fn generate_token() -> String {
    generate_uuid_v7().to_string()
}

fn reg_key(token: &str) -> String {
    format!("passkey:reg:{token}")
}

fn auth_key(token: &str) -> String {
    format!("passkey:auth:{token}")
}

fn two_factor_key(token: &str) -> String {
    format!("passkey:2fa:{token}")
}
