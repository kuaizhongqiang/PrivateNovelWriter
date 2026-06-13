pub mod agent;
pub mod data;

#[derive(Debug, Clone)]
pub enum Command {
    Data(data::DataCommand),
    Agent(agent::AgentCommand),
}
