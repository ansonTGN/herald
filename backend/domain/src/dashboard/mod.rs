mod entities;
mod ports;

pub use entities::{AuthTrendPoint, DashboardStats, UserStats};
pub use ports::DashboardRepository;

#[cfg(test)]
pub use ports::MockDashboardRepository;
