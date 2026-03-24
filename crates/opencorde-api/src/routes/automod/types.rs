//! # AutoMod Types
//! Request and response types for AutoMod rule management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AutomodRuleResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub keywords: Vec<String>,
    pub enabled: bool,
    pub action: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAutomodRuleRequest {
    pub name: Option<String>,
    pub keywords: Vec<String>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAutomodRuleRequest {
    pub name: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub action: Option<String>,
}

impl CreateAutomodRuleRequest {
    /// Validate this request for creating a rule.
    pub fn validate(&self) -> Result<(), String> {
        if self.keywords.is_empty() {
            return Err("add at least one keyword".to_string());
        }

        if self.keywords.len() > 50 {
            return Err("maximum 50 keywords per rule".to_string());
        }

        Ok(())
    }
}

impl UpdateAutomodRuleRequest {
    /// Validate keywords if provided.
    pub fn validate_keywords(&self) -> Result<(), String> {
        if let Some(ref kw_vec) = self.keywords {
            if kw_vec.is_empty() {
                return Err("add at least one keyword".to_string());
            }
            if kw_vec.len() > 50 {
                return Err("maximum 50 keywords per rule".to_string());
            }
        }
        Ok(())
    }
}
