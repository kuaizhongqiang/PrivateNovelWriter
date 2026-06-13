pub mod data;
pub mod agent;

#[derive(Debug, Clone)]
pub enum Command {
    Data(data::DataCommand),
    Agent(agent::AgentCommand),
}
