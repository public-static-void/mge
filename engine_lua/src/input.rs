use std::io::{self, Write};

/// Standard input
pub struct StdinInput;

impl InputProvider for StdinInput {
    /// Reads input from stdin
    fn read_line(&mut self, prompt: &str) -> Result<String, io::Error> {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
}

/// Input provider
pub trait InputProvider {
    /// Reads input
    fn read_line(&mut self, prompt: &str) -> Result<String, std::io::Error>;
}
