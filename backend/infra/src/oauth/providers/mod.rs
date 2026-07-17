// OAuth provider implementations in infrastructure layer
// These implementations use HTTP client abstraction to follow hexagonal architecture

pub mod apple;
pub mod facebook;
pub mod github;
pub mod google;
pub mod wechat;
pub mod wechat_miniprogram;

pub use apple::AppleOAuthProvider;
pub use facebook::FacebookOAuthProvider;
pub use github::GitHubOAuthProvider;
pub use google::GoogleOAuthProvider;
// Re-export so the free function is reachable at
// `herald_core::infrastructure::oauth::verify_google_id_token`. The glob
// `pub use providers::*;` in oauth/mod.rs only lifts items re-exported here;
// it does not auto-promote nested-module free functions.
pub use google::verify_google_id_token;
pub use wechat::WeChatOAuthProvider;
pub use wechat_miniprogram::WeChatMiniProgramProvider;
