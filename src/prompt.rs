use anyhow::{Context, Result};
use std::io::{BufRead, Write};

pub fn prompt(msg: &str) -> Result<String> {
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

pub fn prompt_password(msg: &str) -> Result<String> {
    rpassword::prompt_password(msg).context("failed to read password")
}
