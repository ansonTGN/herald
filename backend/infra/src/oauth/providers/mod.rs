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
pub use wechat::WeChatOAuthProvider;
pub use wechat_miniprogram::WeChatMiniProgramProvider;
