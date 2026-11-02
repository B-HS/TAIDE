use taide_model::agent::DetectedAgent;
use taide_model::ids::ProjectId;

pub trait TerminalAgentSurfaces {
    fn project_agents(&self, _project: &ProjectId) -> Vec<DetectedAgent> {
        Vec::new()
    }
    fn is_agent_status_badge_enabled(&self) -> bool {
        true
    }
}
