use anyhow::{anyhow, Result};
use regex::Regex;
use std::{fmt::Display, path::PathBuf};

pub trait ValidatedValue {
    type V: Display;

    fn get(&self) -> &Self::V;
    fn set(&mut self, value: Self::V);
    fn is_value_valid(value: &Self::V) -> Result<()>;

    fn is_valid(&self) -> Result<()> {
        Self::is_value_valid(self.get())
    }

    fn safe_set(&mut self, value: Self::V) -> Result<()> {
        Self::is_value_valid(&value)?;
        self.set(value);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedDirectory(String);

impl ValidatedDirectory {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl ValidatedValue for ValidatedDirectory {
    type V = String;

    fn get(&self) -> &String {
        &self.0
    }

    fn set(&mut self, value: String) {
        self.0 = value;
    }

    fn is_value_valid(value: &String) -> Result<()> {
        let path = PathBuf::from(value);
        if !path.exists() {
            return Err(anyhow!("Non-existent directory"));
        }
        if !path.is_dir() {
            return Err(anyhow!("Is not directory"));
        }
        Ok(())
    }
}

impl Display for ValidatedDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatedDirectory")
            .field(&self.get())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedPort(u16);

impl ValidatedPort {
    pub fn new(value: u16) -> Self {
        Self(value)
    }
}

impl ValidatedValue for ValidatedPort {
    type V = u16;

    fn get(&self) -> &u16 {
        &self.0
    }

    fn set(&mut self, value: u16) {
        self.0 = value;
    }

    fn is_value_valid(value: &u16) -> Result<()> {
        if *value < 1024 {
            return Err(anyhow!(format!("Invalid port: {}", value)));
        }
        Ok(())
    }
}

impl Display for ValidatedPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatedPort").field(&self.get()).finish()
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedIPv4(String);

impl ValidatedIPv4 {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

impl ValidatedValue for ValidatedIPv4 {
    type V = String;

    fn get(&self) -> &String {
        &self.0
    }

    fn set(&mut self, value: String) {
        self.0 = value;
    }

    fn is_value_valid(value: &String) -> Result<()> {
        if value == "localhost" {
            return Ok(());
        }
        let re = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
        if !re.is_match(value) {
            return Err(anyhow!(format!("Invalid IPv4: {}", value)));
        }
        Ok(())
    }
}

impl Display for ValidatedIPv4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ValidatedIPv4").field(&self.get()).finish()
    }
}
