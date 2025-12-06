pub mod planner;
pub mod retriever;
pub mod summarizer;
pub mod decision;
pub mod action;

// Agent communication channels
pub const PLANNER_CHANNEL: &str = "agent:planner:request";
pub const RETRIEVER_CHANNEL: &str = "agent:retriever:request";
pub const SUMMARIZER_CHANNEL: &str = "agent:summarizer:request";
pub const DECISION_CHANNEL: &str = "agent:decision:request";
pub const ACTION_CHANNEL: &str = "agent:action:request";
