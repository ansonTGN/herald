mod config_service_impl;
mod entities;
pub mod ports;
mod service;

pub use config_service_impl::RealmConfigServiceImpl;
pub use entities::{
    BatchUpsertRealmConfigRequest, ConfigType, CustomDomainConfig, CustomDomainStatus, RealmConfig,
    UpsertRealmConfigRequest, normalize_and_validate_hostname,
};
pub use ports::RealmConfigRepository;
pub use service::RealmConfigService;
