use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, Result};

/// A post-state instruction to the [`App`].
/// 
/// A [`mut Command`] is passed into a state (see: [`App::register_state`]) which is used within the
/// state to trigger special instructions back in the [`App`]. The [`Command`] is handled after the
/// state returns, meaning the state cannot affect the [`App`] before then.
pub enum Command {
    Nothing,
    QueueState(String),
    Exit,
}

impl Command {
    pub fn nothing(&mut self) {
        *self = Command::Nothing;
    }

    pub fn queue_state<S: Into<String>>(&mut self, state_name: S) {
        *self = Command::QueueState(state_name.into());
    }

    pub fn exit(&mut self) {
        *self = Command::Exit
    }
}

enum ControlMode {
    State(String),
    Exit,
}

pub struct App<T> {
    data: Rc<RefCell<T>>,
    states: HashMap<String, Box<dyn Fn(&mut T, &mut Command)>>,
    control_mode: ControlMode,
}

impl<T> App<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Rc::new(RefCell::new(data)),
            states: HashMap::new(),
            control_mode: ControlMode::Exit,
        }
    }

    /// Registers a state for the [`App`].
    /// 
    /// A state is a function that is called every time [`App::update`] is invoked. States are
    /// referenced by their key, or [`state_name`]. A state must have two parameters: [`&mut T`], which
    /// corresponds to the app's universal data, and [`&mut Command`].
    pub fn register_state<S: ToString, F: Fn(&mut T, &mut Command) + 'static>(
        &mut self,
        state_name: S,
        func: F,
    ) {
        self.states.insert(state_name.to_string(), Box::new(func));
    }

    /// [`App`] driver.
    ///
    /// Triggers the queued state through [`trigger_state`] and then handles and then returns a
    /// [`bool`] indicating whether [`update`] should be called again. If the [`App`] should
    /// continue updating, returns [`true`], otherwise [`false`].
    pub fn update(&mut self) -> Result<bool> {
        match &self.control_mode {
            ControlMode::State(state_name) => {
                self.trigger_state(state_name.clone())?;
                Ok(true)
            }
            ControlMode::Exit => Ok(false),
        }
    }

    /// State driver.
    ///
    /// Returns an error if the state has not been registered via [`App::register_state`].
    pub fn trigger_state<S: AsRef<str>>(&mut self, state_name: S) -> Result<()> {
        let func = self.states.get(state_name.as_ref()).ok_or(anyhow!(format!(
            "State '{}' does not exist or is not registered.",
            state_name.as_ref()
        )))?;
        let mut command = Command::Nothing;
        func(&mut Rc::clone(&mut self.data).borrow_mut(), &mut command);

        match command {
            Command::Nothing => (),
            Command::QueueState(state_name) => {
                self.control_mode = ControlMode::State(state_name);
            }
            Command::Exit => {
                self.control_mode = ControlMode::Exit;
            }
        }

        Ok(())
    }

    /// Queue the state to be triggered on the next [`update`].
    pub fn queue_state<S: ToString>(&mut self, state_name: S) {
        let state_name = state_name.to_string();
        self.control_mode = ControlMode::State(state_name);
    }
}
