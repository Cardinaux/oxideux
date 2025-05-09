//! Standardized command line interface actions.
//! 
//!  This module is for standardizing actions related to the command-line interface.

use std::io::{self, Write};
use std::fmt::Display;

use indexmap::IndexMap;

pub fn sep_low() {
    println!("__________");
}

pub fn sep_thin() {
    println!("----------");
}

pub fn sep_thick() {
    println!("==========");
}

pub fn notice<O: Display>(what: O) {
    println!("<(!)> {}", what);
}

pub fn notice_if_some<O: Display>(what: &Option<O>) {
    if let Some(value) = what {
        notice(value);
    }
}

pub fn notice_all<O: Display>(what: &Vec<O>) {
    println!();
    for value in what {
        notice(value);
    }
    println!();
}

pub fn out<O: Display>(what: O) {
    println!("{}", what);
}

pub fn out_if_some<O: Display>(what: &Option<O>) {
    if let Some(value) = what {
        out(value);
    }
}

pub fn clear() {
    for _ in 0..20 {
        println!();
    }
}

pub fn input() -> String {
    print!(">> ");
    io::stdout().flush().expect("Could not flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Could not read from stdin");

    input.trim().to_string()
}

#[derive(Debug)]
pub enum OptionType {
    Dynamic(usize),
    Static(String),
    Error(String),
}

pub struct InputOptions {
    dynamic_options: Vec<String>,
    static_options: IndexMap<String, String>,
    header_dynamic: Option<String>,
    header_static: Option<String>,
}

impl InputOptions {
    pub fn new() -> Self {
        Self {
            dynamic_options: vec![],
            static_options: IndexMap::new(),
            header_dynamic: None,
            header_static: None,
        }
    }

    pub fn add_dynamic<S: ToString>(&mut self, label: S) -> &mut Self {
        self.dynamic_options.push(label.to_string());
        self
    }

    pub fn add_static<K: ToString, V: ToString>(&mut self, key: K, label: V) -> &mut Self {
        self.static_options.insert(key.to_string(), label.to_string());
        self
    }

    pub fn set_header_dynamic<S: ToString>(&mut self, what: S) -> &mut Self {
        self.header_dynamic = Some(what.to_string());
        self
    }

    pub fn set_header_static<S: ToString>(&mut self, what: S) -> &mut Self {
        self.header_static = Some(what.to_string());
        self
    }

    /// Queries [`stdin`] for an input, then converts it to an [`OptionType`]
    pub fn get(&self) -> OptionType {
        if self.dynamic_options.len() > 0 {
            out_if_some(&self.header_dynamic);
            for (key, label) in self.dynamic_options.iter().enumerate() {
                out(format!("{} :: {}", key, label));
            }
        }

        if self.static_options.len() > 0 {
            out_if_some(&self.header_static);
            for (key, label) in &self.static_options {
                out(format!("[{}] {}", key, label));
            }
        }

        let option = input();
        
        // First try to resolve it as a static option
        if self.static_options.contains_key(&option) {
            return OptionType::Static(option);
        }

        // Then try to resolve it as a dynamic option
        if let Ok(value) = option.parse::<usize>() {
            if value < self.dynamic_options.len() {
                return OptionType::Dynamic(value)
            }
        }

        OptionType::Error(format!("'{}' is not a valid option.", option))
    }
}