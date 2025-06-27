use std::io::{self, Write};

pub struct StdinInput;

impl InputProvider for StdinInput {
    fn read_line(&mut self, prompt: &str) -> Result<String, io::Error> {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
}

pub trait InputProvider {
    fn read_line(&mut self, prompt: &str) -> Result<String, std::io::Error>;
}
