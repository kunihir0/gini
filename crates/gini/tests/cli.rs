use assert_cmd::Command; // Bring Command into scope
use predicates::prelude::*; // Bring predicate traits into scope

#[test]
fn test_ping_command() -> Result<(), Box<dyn std::error::Error>> {
    // Get the binary command for the 'gini' crate
    let mut cmd = Command::cargo_bin("gini")?;

    // Run the command with the --ping argument
    cmd.arg("--ping");

    // Assert that the command runs successfully
    // and that its standard output contains "pong"
    cmd.assert()
        .success() // Check for exit code 0
        .stdout(predicate::str::contains("pong")); // Check stdout for "pong"

    Ok(())
}

#[test]
fn test_no_args_runs_normally() -> Result<(), Box<dyn std::error::Error>> {
    // Test that running without args doesn't print "pong" and includes normal startup/shutdown messages
    let mut cmd = Command::cargo_bin("gini")?;

    cmd.assert()
        .success() // Should still succeed
        .stdout(predicate::str::contains("Initializing application...")) // Check for normal startup message
        .stdout(predicate::str::contains("Shutting down application...")) // Check for normal shutdown message
        .stdout(predicate::str::contains("pong").not()); // Ensure "pong" is NOT printed

    Ok(())
}