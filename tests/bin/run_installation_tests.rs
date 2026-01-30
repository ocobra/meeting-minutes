use std::process::{Command, exit};
use std::env;

fn main() {
    println!("Running Meetily Installation Guide Property Tests...");
    
    // Change to the tests directory
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let tests_dir = current_dir.join("tests");
    
    if !tests_dir.exists() {
        eprintln!("Tests directory not found. Please run from the project root.");
        exit(1);
    }
    
    env::set_current_dir(&tests_dir).expect("Failed to change to tests directory");
    
    // Run the property tests
    let output = Command::new("cargo")
        .args(&["test", "--", "--nocapture"])
        .output()
        .expect("Failed to execute cargo test");
    
    // Print the output
    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Exit with the same code as the test command
    exit(output.status.code().unwrap_or(1));
}