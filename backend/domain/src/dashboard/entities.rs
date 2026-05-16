use serde::{Deserialize, Serialize};

/// User statistics for a realm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub total_users: i64,
    pub new_users: i64,
    pub active_users: i64,
}

/// A single data point in the authentication trend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTrendPoint {
    pub date: String,
    pub success_count: i64,
    pub failure_count: i64,
}

/// Aggregated dashboard statistics for a realm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub user_stats: UserStats,
    pub auth_trend: Vec<AuthTrendPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_stats_serializes_to_camel_case() {
        let stats = UserStats {
            total_users: 100,
            new_users: 10,
            active_users: 50,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"totalUsers\""));
        assert!(json.contains("\"newUsers\""));
        assert!(json.contains("\"activeUsers\""));
    }

    #[test]
    fn auth_trend_point_serializes_to_snake_case() {
        let point = AuthTrendPoint {
            date: "2026-05-16".to_string(),
            success_count: 42,
            failure_count: 3,
        };
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("\"success_count\""));
        assert!(json.contains("\"failure_count\""));
    }

    #[test]
    fn dashboard_stats_roundtrip() {
        let stats = DashboardStats {
            user_stats: UserStats {
                total_users: 128,
                new_users: 12,
                active_users: 34,
            },
            auth_trend: vec![AuthTrendPoint {
                date: "2026-05-16".to_string(),
                success_count: 80,
                failure_count: 5,
            }],
        };
        let json = serde_json::to_string(&stats).unwrap();
        let back: DashboardStats = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_stats.total_users, 128);
        assert_eq!(back.auth_trend.len(), 1);
        assert_eq!(back.auth_trend[0].date, "2026-05-16");
    }
}
