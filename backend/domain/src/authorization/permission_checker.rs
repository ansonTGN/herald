// DELETED: This file violated hexagonal architecture
// - Had SeaORM dependencies in domain layer
// - Duplicated Policy struct from permission_service.rs
// - Functionality moved to infrastructure/authorization/redis_permission_checker.rs
// See: backend/core/src/infrastructure/authorization/redis_permission_checker.rs
