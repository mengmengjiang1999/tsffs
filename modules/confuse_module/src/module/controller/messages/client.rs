//! Messages generated by the client

use serde::{Deserialize, Serialize};

use crate::{module::config::InitializeConfig, state::ConfuseModuleInput};

use super::Message;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    /// Initialize event, the fuzzer signals the Confuse SIMICS module to initialize itself with
    /// a given set of global campaign settings
    Initialize(InitializeConfig),
    /// The fuzzer signals the Confuse SIMICS module to run with a given input of bytes
    Run(Vec<u8>),
    /// The fuzzer signals the Confuse SIMICS module to reset to the start snapshot
    Reset,
    /// The fuzzer signals the Confuse SIMICS module to stop execution and exit
    Exit,
}
