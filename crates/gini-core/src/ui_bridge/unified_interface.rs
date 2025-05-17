use crate::ui_bridge::error::UiBridgeError;
use crate::ui_bridge::{UiMessage, UserInput};
use std::fmt::Debug;

/// A unified trait for UI implementations, combining capabilities for message handling,
/// user input, and lifecycle management.
///
/// Implementors of this trait can be registered with the `UnifiedUiManager` to
/// interact with the core application.
pub trait UnifiedUiInterface: Send + Sync + Debug {
    /// Returns the unique name of the UI interface (e.g., "cli", "web_v1").
    ///
    /// This name is used for identification and management purposes.
    fn name(&self) -> &str;

    /// Initializes the UI interface.
    ///
    /// This method is called once by the `UnifiedUiManager` before any other
    /// operations are performed on the interface. Implementations should perform
    /// any necessary setup here.
    fn initialize(&mut self) -> Result<(), UiBridgeError>;

    /// Handles a message sent from the core application to the UI.
    ///
    /// Implementations should display this message to the user in an appropriate
    /// manner. This method can be stateful if the UI needs to update its
    /// internal state based on the message.
    fn handle_message(&mut self, message: &UiMessage) -> Result<(), UiBridgeError>;

    /// Sends user input received from the UI to the core application.
    ///
    /// This method is intended to be called by the UI implementation itself when
    /// user input is available. The `UnifiedUiManager` will then route this input
    /// to the appropriate core system (e.g., an event bus or input handler).
    ///
    /// # Arguments
    ///
    /// * `input`: The `UserInput` captured by the UI.
    fn send_input(&mut self, input: UserInput) -> Result<(), UiBridgeError>;

    /// Called periodically or on demand to update the UI's state or display.
    ///
    /// This can be used for UIs that require regular refreshes or updates
    /// independent of direct messages from the core.
    fn update(&mut self) -> Result<(), UiBridgeError>;

    /// Finalizes and cleans up the UI interface before shutdown.
    ///
    /// This method is called once by the `UnifiedUiManager` when the application
    /// is shutting down. Implementations should perform any necessary cleanup
    /// or resource deallocation here.
    fn finalize(&mut self) -> Result<(), UiBridgeError>;

    /// Checks if this UI interface supports interactive mode.
    ///
    /// Interactive mode typically means the UI can accept `UserInput`.
    ///
    /// # Returns
    ///
    /// * `true` if the interface supports interactive input, `false` otherwise.
    fn supports_interactive(&self) -> bool;
}