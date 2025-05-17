use gini_core::ui_bridge::{UnifiedUiInterface, UiMessage, UserInput, error::UiBridgeError, MessageSeverity};

/// A basic UI interface for the command-line interface.
///
/// This interface handles messages by printing them to the console
/// and can send user input back to the core.
#[derive(Debug)]
pub struct CliInterface;

impl UnifiedUiInterface for CliInterface {
    /// Returns the name of this interface.
    fn name(&self) -> &str {
        "cli"
    }

    /// Initializes the CLI interface.
    ///
    /// Currently, this is a no-op.
    fn initialize(&mut self) -> Result<(), UiBridgeError> {
        println!("[CLI] Interface initialized.");
        Ok(())
    }

    /// Handles incoming messages from the core application.
    ///
    /// It prints the message to standard output, with basic formatting.
    fn handle_message(&mut self, message: &UiMessage) -> Result<(), UiBridgeError> {
        let severity_prefix = match message.update_type {
            gini_core::ui_bridge::UiUpdateType::Log(_, severity) => match severity {
                MessageSeverity::Error | MessageSeverity::Critical => "[ERROR] ",
                MessageSeverity::Warning => "[WARN]  ",
                MessageSeverity::Info => "[INFO]  ",
                MessageSeverity::Debug => "[DEBUG] ",
            },
            _ => "[INFO]  ", // Default for other message types like Status, Progress, Dialog
        };
        println!("{} {}: {:?}", severity_prefix, message.source, message.update_type);
        Ok(())
    }

    /// Sends user input from the UI to the core application.
    ///
    /// This implementation reads a line from stdin.
    /// Note: In a real application, this might be part of an input loop
    /// or triggered by specific commands. The `UnifiedUiManager` would typically
    /// call this on an active interactive provider.
    /// For this refactor, we assume this method is called when input is expected.
    fn send_input(&mut self, input_request: UserInput) -> Result<(), UiBridgeError> {
        // This method on the trait is for the UI to *send* input it has captured.
        // The `input_request` parameter here is a bit confusing in this context.
        // Let's assume for now this method is called by some external CLI input loop,
        // and the `input_request` is what was gathered.
        // Or, if the core *requests* input, this method isn't the one called.
        // The plan says: "The unified manager would then route this input appropriately".
        // "This method would be called by the UI implementation when user input is available."
        // So, this `CliInterface` would need its own way to gather input (e.g. read_line)
        // and then it would call a method on the `UnifiedUiManager` like `submit_user_input`.
        // The `send_input` on the trait itself seems to be for the *manager* to push input *to* the UI,
        // which is not its primary role as per the plan's description of `send_input`.
        //
        // Re-interpreting based on plan: `fn send_input(&mut self, input: UserInput) -> Result<(), UiBridgeError>;`
        // This implies the UI has ALREADY GATHERED the `input` and is now sending it.
        // So, this method itself doesn't read from stdin. Some other part of `CliInterface` would.
        // For now, let's make it a placeholder that indicates it was called.
        // The actual reading of input and calling manager.submit_user_input() would be elsewhere.

        println!("[CLI] send_input called with: {:?}. This CLI implementation would have gathered this input and is now 'sending' it (conceptually).", input_request);
        // In a real scenario, this method might not be directly called by the manager.
        // Instead, the CLI's own input loop would capture UserInput and then call
        // `manager.submit_user_input(captured_input, self.name())`.
        // This trait method `send_input` is slightly ambiguous in its role for a provider.
        // Let's assume it's for when the manager *pushes* an input event *to* the UI,
        // which is less common.
        // For now, we'll treat it as a simple acknowledgement.
        Ok(())
    }


    /// Updates the CLI interface.
    ///
    /// Currently, this is a no-op as the CLI updates reactively to messages.
    fn update(&mut self) -> Result<(), UiBridgeError> {
        // No explicit update phase for a simple CLI.
        Ok(())
    }

    /// Finalizes the CLI interface.
    ///
    /// Currently, this is a no-op.
    fn finalize(&mut self) -> Result<(), UiBridgeError> {
        println!("[CLI] Interface finalized.");
        Ok(())
    }

    /// Checks if the CLI interface supports interactive mode.
    fn supports_interactive(&self) -> bool {
        true // CLI is typically interactive
    }
}

// Removed unused request_text_input helper function
// impl CliInterface {
//     /// Helper function to prompt user and get text input.
//     /// This is an example of how CliInterface might gather input,
//     /// which would then be passed to `manager.submit_user_input`.
//     pub fn request_text_input(prompt_message: &str) -> io::Result<String> {
//         print!("{}", prompt_message);
//         io::stdout().flush()?;
//         let mut buffer = String::new();
//         io::stdin().read_line(&mut buffer)?;
//         Ok(buffer.trim().to_string())
//     }
// }