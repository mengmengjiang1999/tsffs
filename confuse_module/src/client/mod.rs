//! The CONFUSE module client provides a common client-side controller for a fuzzer or other tool
//! to communicate with the module while keeping consistent with the state machine the module
//! implements.
//!
//! This client is designed to be used with the [`confuse-fuzz`] crate, but can be used manually as
//! well to implement bespoke systems.
//!
//! # Examples
//!
//! In this example, we show what an extremely basic fuzz loop might look like
//! without using LibAFL. This loop is consistent with the state machine used
//! internally by the client and module that keeps them in sync.
//!
//! ```text
//! use anyhow::Result;
//! use std::path::PathBuf;
//! use confuse_simics_project::SimicsProject;
//! use confuse_module::{
//!     client::Client,
//!     config::InputConfig,
//!     traits::ConfuseClient,
//! };
//!
//! fn main() -> Result<()> {
//!     let simics_script_path = PathBuf::from("./script.simics");
//!     let project = SimicsProject::try_new_latest()?
//!         // Add a file to the created simics project at `PROJECT_ROOT/scripts/script.simics`
//!         .try_with_file(&simics_script_path, "scripts/script.simics")?
//!         // This script will be our entrypoint when we run SIMICS
//!         .try_with_file_argument("scripts/script.simics")?;
//!
//!     // Create a client that owns the project we just created
//!     let mut client = Client::try_new(project)?;
//!
//!     // Create a blank configuration
//!     let config = InputConfig::default();
//!
//!     // Initialize the client. This takes us up to the point where the module is ready
//!     // to start the fuzzing loop
//!     let output_config = client.initialize(config)?;
//!
//!
//!     for _ in 0..100 {
//!         // Reset the target to its initial state once it has been initialized. We could also
//!         client.reset()?;
//!         // Run the target with the same input every time. In real life, we want to
//!         // swap this out with a fuzzer, of course
//!         let stop_reason = client.run(vec![0x41; 64])?;
//!     }
//!
//!     // Cleanly exit SIMICS and stop the client
//!     client.exit()?;
//!
//!     Ok(())
//! }
//! ```

use crate::{
    config::{InputConfig, OutputConfig},
    messages::{client::ClientMessage, module::ModuleMessage},
    state::State,
    stops::StopReason,
    traits::ConfuseClient,
};
use anyhow::{bail, Result};
use std::sync::mpsc::{Receiver, Sender};
use tracing::{debug, info};

/// The client for the CONFUSE module. Allows controlling the module over IPC using the child
/// process spawned by a running project.
pub struct Client {
    /// State machine to keep track of the current state between the client and module
    state: State,
    /// Transmit end of IPC message channel between client and module
    tx: Sender<ClientMessage>,
    /// Receive end of IPC message channel between client and module
    rx: Receiver<ModuleMessage>,
}

impl Client {
    /// Try to initialize a `Client` from a built `SimicsProject` on disk, which should include
    /// the CONFUSE module and may have additional configuration according to user needs. Creating
    /// the client will start the SIMICS project, which should be configured as necessary *before*
    /// passing it into this constructor.
    ///
    /// The CONFUSE Simics module will be added to the project for you if it is not present,
    /// so
    pub fn new(tx: Sender<ClientMessage>, rx: Receiver<ModuleMessage>) -> Self {
        Self {
            state: State::new(),
            tx,
            rx,
        }
    }
}

impl ConfuseClient for Client {
    /// Initialize the client with a configuration. The client will return an output
    /// configuration which contains various information the SIMICS module needs to
    /// inform the client of, including memory maps for coverage. Changes the
    /// internal state from `Uninitialized` to `HalfInitialized` and then from
    /// `HalfInitialized` to `ConfuseModuleState::Initialized`.
    fn initialize(&mut self, config: InputConfig) -> Result<OutputConfig> {
        info!("Sending initialize message");
        self.send_msg(ClientMessage::Initialize(config))?;

        info!("Waiting for initialized message");
        if let ModuleMessage::Initialized(config) = self.recv_msg()? {
            Ok(config)
        } else {
            bail!("Initialization failed, received unexpected message");
        }
    }

    /// Reset the module to the beginning of the fuzz loop (the state as snapshotted).
    /// Changes the internal state from `Stopped` or `Initialized` to `HalfReady`, then
    /// from `HalfReady` to `Ready`.
    fn reset(&mut self) -> Result<()> {
        debug!("Sending reset message");
        self.send_msg(ClientMessage::Reset)?;

        debug!("Waiting for ready message");
        if let ModuleMessage::Ready = self.recv_msg()? {
            Ok(())
        } else {
            bail!("Reset failed, received unexpected message");
        }
    }

    /// Signal the module to run the target software. Changes the intenal state from `Ready` to
    /// `Running`, then once the run finishes either with a normal stop, a timeout, or a crash,
    /// from `Running` to `Stopped`. This function blocks until the target software stops and the
    /// module detects it, so it may take a long time or if there is an unexpected bug it may
    /// hang.
    fn run(&mut self, input: Vec<u8>) -> Result<StopReason> {
        debug!("Sending run message");
        self.send_msg(ClientMessage::Run(input))?;

        debug!("Waiting for stopped message");
        if let ModuleMessage::Stopped(reason) = self.recv_msg()? {
            Ok(reason)
        } else {
            bail!("Run failed, received unexpected message");
        }
    }

    /// Signal the module to exit SIMICS, stopping the fuzzing process. Changes the internal state
    /// from any state to `Done`.
    fn exit(&mut self) -> Result<()> {
        info!("Sending exit message");
        self.send_msg(ClientMessage::Exit)?;

        Ok(())
    }

    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    fn rx_mut(&mut self) -> &mut Receiver<ModuleMessage> {
        &mut self.rx
    }

    fn tx_mut(&mut self) -> &mut Sender<ClientMessage> {
        &mut self.tx
    }
}
