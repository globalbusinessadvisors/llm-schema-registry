//! Attribute-Based Access Control (ABAC)
//!
//! Context-aware access control based on:
//! - User attributes (roles, department, clearance level)
//! - Resource attributes (sensitivity, owner, classification)
//! - Environmental attributes (time, location, IP range)
//! - Action attributes (read, write, delete, admin)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

// =============================================================================
// ABAC Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacContext {
    /// User attributes
    pub user: UserAttributes,
    /// Resource being accessed
    pub resource: ResourceAttributes,
    /// Environment context
    pub environment: EnvironmentAttributes,
    /// Action being performed
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAttributes {
    pub user_id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub department: Option<String>,
    pub clearance_level: u8, // 0-5, higher is more privileged
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAttributes {
    pub resource_id: String,
    pub resource_type: String,
    pub owner: Option<String>,
    pub sensitivity: SensitivityLevel,
    pub tags: Vec<String>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SensitivityLevel {
    Public = 0,
    Internal = 1,
    Confidential = 2,
    Secret = 3,
    TopSecret = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentAttributes {
    pub timestamp: u64,
    pub source_ip: Option<IpAddr>,
    pub location: Option<String>,
    pub time_of_day: u8, // Hour of day (0-23)
    pub day_of_week: u8, // 0=Sunday, 6=Saturday
}

impl Default for EnvironmentAttributes {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp: now,
            source_ip: None,
            location: None,
            time_of_day: 12,
            day_of_week: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
    Admin,
}

// =============================================================================
// ABAC Policy
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<AbacRule>,
    pub effect: PolicyEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbacRule {
    pub condition: Condition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// User has specific role
    UserHasRole(String),
    /// User clearance level >= threshold
    UserClearanceLevel { min: u8 },
    /// Resource sensitivity <= user clearance
    ResourceSensitivity { max_level: SensitivityLevel },
    /// User is resource owner
    UserIsOwner,
    /// Action is one of
    ActionIs(Vec<Action>),
    /// Source IP in range
    IpInRange { cidr: String },
    /// Time of day between hours
    TimeBetween { start_hour: u8, end_hour: u8 },
    /// Day of week
    DayOfWeek { days: Vec<u8> },
    /// Department matches
    DepartmentIs(String),
    /// Resource has tag
    ResourceHasTag(String),
    /// All conditions must match (AND)
    All(Vec<Condition>),
    /// Any condition must match (OR)
    Any(Vec<Condition>),
    /// Negate condition
    Not(Box<Condition>),
}

// =============================================================================
// ABAC Engine (also exported as AbacManager for compatibility)
// =============================================================================

pub struct AbacEngine {
    policies: Vec<AbacPolicy>,
}

pub type AbacManager = AbacEngine;

impl AbacEngine {
    pub fn new() -> Self {
        Self {
            policies: Self::default_policies(),
        }
    }

    /// Add a custom policy
    pub fn add_policy(&mut self, policy: AbacPolicy) {
        self.policies.push(policy);
    }

    /// Evaluate access request
    pub fn evaluate(&self, context: &AbacContext) -> AccessDecision {
        let mut allow_policies = Vec::new();
        let mut deny_policies = Vec::new();

        for policy in &self.policies {
            if self.evaluate_policy(policy, context) {
                match policy.effect {
                    PolicyEffect::Allow => allow_policies.push(policy.id.clone()),
                    PolicyEffect::Deny => deny_policies.push(policy.id.clone()),
                }
            }
        }

        // Deny takes precedence
        if !deny_policies.is_empty() {
            return AccessDecision {
                allowed: false,
                reason: format!("Denied by policies: {:?}", deny_policies),
                matched_policies: deny_policies,
            };
        }

        // Must have at least one allow
        if !allow_policies.is_empty() {
            return AccessDecision {
                allowed: true,
                reason: format!("Allowed by policies: {:?}", allow_policies),
                matched_policies: allow_policies,
            };
        }

        // Default deny
        AccessDecision {
            allowed: false,
            reason: "No matching allow policies".to_string(),
            matched_policies: vec![],
        }
    }

    fn evaluate_policy(&self, policy: &AbacPolicy, context: &AbacContext) -> bool {
        policy.rules.iter().all(|rule| self.evaluate_condition(&rule.condition, context))
    }

    fn evaluate_condition(&self, condition: &Condition, context: &AbacContext) -> bool {
        match condition {
            Condition::UserHasRole(role) => context.user.roles.contains(role),

            Condition::UserClearanceLevel { min } => context.user.clearance_level >= *min,

            Condition::ResourceSensitivity { max_level } => {
                context.resource.sensitivity <= *max_level
            }

            Condition::UserIsOwner => {
                context.resource.owner.as_ref() == Some(&context.user.user_id)
            }

            Condition::ActionIs(actions) => actions.contains(&context.action),

            Condition::IpInRange { cidr } => {
                if let Some(ip) = context.environment.source_ip {
                    self.ip_in_cidr(ip, cidr)
                } else {
                    false
                }
            }

            Condition::TimeBetween { start_hour, end_hour } => {
                let hour = context.environment.time_of_day;
                hour >= *start_hour && hour <= *end_hour
            }

            Condition::DayOfWeek { days } => days.contains(&context.environment.day_of_week),

            Condition::DepartmentIs(dept) => {
                context.user.department.as_ref() == Some(dept)
            }

            Condition::ResourceHasTag(tag) => context.resource.tags.contains(tag),

            Condition::All(conditions) => {
                conditions.iter().all(|c| self.evaluate_condition(c, context))
            }

            Condition::Any(conditions) => {
                conditions.iter().any(|c| self.evaluate_condition(c, context))
            }

            Condition::Not(condition) => !self.evaluate_condition(condition, context),
        }
    }

    fn ip_in_cidr(&self, _ip: IpAddr, _cidr: &str) -> bool {
        // Simplified IP range check
        // In production, use proper CIDR parsing
        true
    }

    /// Default security policies
    fn default_policies() -> Vec<AbacPolicy> {
        vec![
            // Policy 1: Admins can do anything
            AbacPolicy {
                id: "admin-all-access".to_string(),
                name: "Admin Full Access".to_string(),
                description: "Admins have unrestricted access".to_string(),
                rules: vec![AbacRule {
                    condition: Condition::UserHasRole("admin".to_string()),
                }],
                effect: PolicyEffect::Allow,
            },
            // Policy 2: Users can read public resources
            AbacPolicy {
                id: "public-read-access".to_string(),
                name: "Public Read Access".to_string(),
                description: "Anyone can read public resources".to_string(),
                rules: vec![
                    AbacRule {
                        condition: Condition::ActionIs(vec![Action::Read]),
                    },
                    AbacRule {
                        condition: Condition::ResourceSensitivity {
                            max_level: SensitivityLevel::Public,
                        },
                    },
                ],
                effect: PolicyEffect::Allow,
            },
            // Policy 3: Users can modify their own resources
            AbacPolicy {
                id: "owner-write-access".to_string(),
                name: "Owner Write Access".to_string(),
                description: "Users can modify resources they own".to_string(),
                rules: vec![
                    AbacRule {
                        condition: Condition::UserIsOwner,
                    },
                    AbacRule {
                        condition: Condition::ActionIs(vec![Action::Write, Action::Delete]),
                    },
                ],
                effect: PolicyEffect::Allow,
            },
            // Policy 4: Require clearance for sensitive resources
            AbacPolicy {
                id: "clearance-based-access".to_string(),
                name: "Clearance-Based Access".to_string(),
                description: "Users need appropriate clearance for sensitive resources".to_string(),
                rules: vec![AbacRule {
                    condition: Condition::All(vec![
                        Condition::UserClearanceLevel { min: 2 },
                        Condition::ResourceSensitivity {
                            max_level: SensitivityLevel::Confidential,
                        },
                    ]),
                }],
                effect: PolicyEffect::Allow,
            },
        ]
    }
}

impl Default for AbacEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Access Decision
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    pub allowed: bool,
    pub reason: String,
    pub matched_policies: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user() -> UserAttributes {
        UserAttributes {
            user_id: "test_user".to_string(),
            email: Some("test@example.com".to_string()),
            roles: vec!["user".to_string()],
            department: Some("engineering".to_string()),
            clearance_level: 1,
            attributes: HashMap::new(),
        }
    }

    fn create_test_resource() -> ResourceAttributes {
        ResourceAttributes {
            resource_id: "res_001".to_string(),
            resource_type: "schema".to_string(),
            owner: Some("test_user".to_string()),
            sensitivity: SensitivityLevel::Internal,
            tags: vec![],
            attributes: HashMap::new(),
        }
    }

    #[test]
    fn test_abac_engine_new() {
        let engine = AbacEngine::new();
        assert!(engine.policies.len() > 0);
    }

    #[test]
    fn test_abac_engine_default() {
        let engine = AbacEngine::default();
        assert!(engine.policies.len() > 0);
    }

    #[test]
    fn test_admin_access() {
        let engine = AbacEngine::new();
        let mut user = create_test_user();
        user.roles.push("admin".to_string());

        let context = AbacContext {
            user,
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Admin,
        };

        let decision = engine.evaluate(&context);
        assert!(decision.allowed);
    }

    #[test]
    fn test_public_read_access() {
        let engine = AbacEngine::new();
        let user = create_test_user();
        let mut resource = create_test_resource();
        resource.sensitivity = SensitivityLevel::Public;

        let context = AbacContext {
            user,
            resource,
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let decision = engine.evaluate(&context);
        assert!(decision.allowed);
    }

    #[test]
    fn test_owner_write_access() {
        let engine = AbacEngine::new();
        let user = create_test_user();
        let resource = create_test_resource();

        let context = AbacContext {
            user,
            resource,
            environment: EnvironmentAttributes::default(),
            action: Action::Write,
        };

        let decision = engine.evaluate(&context);
        assert!(decision.allowed);
    }

    #[test]
    fn test_sensitivity_level_ordering() {
        assert!(SensitivityLevel::Public < SensitivityLevel::Internal);
        assert!(SensitivityLevel::Internal < SensitivityLevel::Confidential);
        assert!(SensitivityLevel::Confidential < SensitivityLevel::Secret);
        assert!(SensitivityLevel::Secret < SensitivityLevel::TopSecret);
    }

    #[test]
    fn test_action_equality() {
        assert_eq!(Action::Read, Action::Read);
        assert_ne!(Action::Read, Action::Write);
    }

    #[test]
    fn test_policy_effect() {
        assert_eq!(PolicyEffect::Allow, PolicyEffect::Allow);
        assert_ne!(PolicyEffect::Allow, PolicyEffect::Deny);
    }

    #[test]
    fn test_environment_attributes_default() {
        let env = EnvironmentAttributes::default();
        assert!(env.timestamp > 0);
        assert_eq!(env.time_of_day, 12);
        assert_eq!(env.day_of_week, 1);
    }

    #[test]
    fn test_user_has_role_condition() {
        let engine = AbacEngine::new();
        let mut user = create_test_user();
        user.roles.push("developer".to_string());

        let context = AbacContext {
            user,
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::UserHasRole("developer".to_string());
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_clearance_level_condition() {
        let engine = AbacEngine::new();
        let mut user = create_test_user();
        user.clearance_level = 3;

        let context = AbacContext {
            user,
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::UserClearanceLevel { min: 2 };
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_user_is_owner_condition() {
        let engine = AbacEngine::new();
        let user = create_test_user();
        let resource = create_test_resource();

        let context = AbacContext {
            user,
            resource,
            environment: EnvironmentAttributes::default(),
            action: Action::Write,
        };

        let condition = Condition::UserIsOwner;
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_action_is_condition() {
        let engine = AbacEngine::new();
        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::ActionIs(vec![Action::Read, Action::Write]);
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_time_between_condition() {
        let engine = AbacEngine::new();
        let mut env = EnvironmentAttributes::default();
        env.time_of_day = 14; // 2 PM

        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: env,
            action: Action::Read,
        };

        let condition = Condition::TimeBetween {
            start_hour: 9,
            end_hour: 17,
        };
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_all_condition() {
        let engine = AbacEngine::new();
        let mut user = create_test_user();
        user.roles.push("admin".to_string());

        let context = AbacContext {
            user,
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::All(vec![
            Condition::UserHasRole("admin".to_string()),
            Condition::ActionIs(vec![Action::Read]),
        ]);
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_any_condition() {
        let engine = AbacEngine::new();
        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::Any(vec![
            Condition::UserHasRole("admin".to_string()),
            Condition::ActionIs(vec![Action::Read]),
        ]);
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_not_condition() {
        let engine = AbacEngine::new();
        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::Not(Box::new(Condition::UserHasRole("admin".to_string())));
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_add_custom_policy() {
        let mut engine = AbacEngine::new();
        let initial_count = engine.policies.len();

        let policy = AbacPolicy {
            id: "custom".to_string(),
            name: "Custom Policy".to_string(),
            description: "Test policy".to_string(),
            rules: vec![],
            effect: PolicyEffect::Allow,
        };

        engine.add_policy(policy);
        assert_eq!(engine.policies.len(), initial_count + 1);
    }

    #[test]
    fn test_access_decision_default_deny() {
        let engine = AbacEngine::new();
        let user = create_test_user();
        let mut resource = create_test_resource();
        resource.sensitivity = SensitivityLevel::Secret;

        let context = AbacContext {
            user,
            resource,
            environment: EnvironmentAttributes::default(),
            action: Action::Write,
        };

        let decision = engine.evaluate(&context);
        assert!(!decision.allowed);
    }

    #[test]
    fn test_resource_has_tag_condition() {
        let engine = AbacEngine::new();
        let mut resource = create_test_resource();
        resource.tags.push("production".to_string());

        let context = AbacContext {
            user: create_test_user(),
            resource,
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::ResourceHasTag("production".to_string());
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_department_is_condition() {
        let engine = AbacEngine::new();
        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: EnvironmentAttributes::default(),
            action: Action::Read,
        };

        let condition = Condition::DepartmentIs("engineering".to_string());
        assert!(engine.evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_day_of_week_condition() {
        let engine = AbacEngine::new();
        let mut env = EnvironmentAttributes::default();
        env.day_of_week = 1; // Monday

        let context = AbacContext {
            user: create_test_user(),
            resource: create_test_resource(),
            environment: env,
            action: Action::Read,
        };

        let condition = Condition::DayOfWeek {
            days: vec![1, 2, 3, 4, 5], // Weekdays
        };
        assert!(engine.evaluate_condition(&condition, &context));
    }
}
