// Planner Agent: Decomposes user queries into executable steps

use crate::models::PlannerStep;
use anyhow::Result;
use tracing::info;

pub struct PlannerAgent {
    llm_endpoint: String,
    api_key: Option<String>,
}

impl PlannerAgent {
    pub fn new(llm_endpoint: String, api_key: Option<String>) -> Self {
        Self { llm_endpoint, api_key }
    }

    pub async fn plan(&self, query: &str) -> Result<Vec<PlannerStep>> {
        info!("Planner: Decomposing query: {}", query);
        
        // TODO: Call LLM with prompt template
        // TODO: Parse structured output
        // TODO: Log to audit trail
        
        // Placeholder
        Ok(vec![
            PlannerStep {
                step: 1,
                action: "retrieve".to_string(),
                args: serde_json::json!({"query": query}),
            },
            PlannerStep {
                step: 2,
                action: "summarize".to_string(),
                args: serde_json::json!({}),
            },
        ])
    }
}
