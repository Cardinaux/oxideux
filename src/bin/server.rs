use std::net::{Shutdown, TcpListener};
use std::path::PathBuf;
use std::process::Command;

use oxideux_rs::app;
use oxideux_rs::cli;
use oxideux_rs::config::{self, ServerProfile};
use oxideux_rs::connection::Connection;
use oxideux_rs::parity;
use oxideux_rs::request::{Request, RequestResult};
use oxideux_rs::validated_values::ValidatedValue;

use anyhow::{self, Result};

#[derive(Default)]
struct AppData {
    profile_names: Vec<String>,
    current_profile: Option<ServerProfile>,
    notices: Vec<String>,
}

impl AppData {
    fn push_notice<S: ToString>(&mut self, message: S) {
        self.notices.push(message.to_string());
    }

    fn clear_notices(&mut self) {
        self.notices.clear();
    }

    fn refresh_cli(&mut self) {
        cli::clear();
        cli::notice_all(&self.notices);
        self.clear_notices();    
    }

    fn refresh_profile_names(&mut self) {
        self.profile_names = config::server::get_profile_names().expect("Something went wrong when refreshing profile names");
    }
}

fn main() -> Result<()> {
    config::server::init_config_file()?;

    let app_data = AppData::default();

    let mut app = app::App::new(app_data);
    app.register_state("pick_profile", state_pick_profile);
    app.register_state("manage_profile", state_manage_profile);
    app.register_state("change_name", state_change_name);
    app.register_state("change_parity_root", state_change_parity_root);
    app.register_state("change_port", state_change_port);
    app.register_state("change_mask", state_change_mask);
    app.register_state("save_updated_profile", state_save_updated_profile);
    app.register_state("start_server", state_start_server);

    app.queue_state("pick_profile");

    while match app.update() {
        Ok(running) => running,
        Err(e) => return Err(e),
    } {}

    Ok(())
}

fn state_pick_profile(app_data: &mut AppData, command: &mut app::Command) {
    app_data.refresh_profile_names();
    app_data.refresh_cli();
    
    let mut options = cli::InputOptions::new();
    
    // Headers
    options
        .set_header_dynamic("PICK A PROFILE:")
        .set_header_static("__________");

    // Add profile names
    for profile_name in &app_data.profile_names {
        options.add_dynamic(profile_name);
    }

    // Add controls
    options 
        .add_static("a", "Create new profile")
        .add_static("r", "Refresh profiles")
        .add_static("c", "Open config directory")
        .add_static("q", "Terminate program");

    match options.get() {
        cli::OptionType::Dynamic(index) => {
            let profile_name = &app_data.profile_names[index];
            let profile = config::server::get_profile(profile_name).unwrap();
            app_data.current_profile = Some(profile);
            command.queue_state("manage_profile");
        },
        cli::OptionType::Static(key) => match key.as_str() {
            "a" => {
                let count = app_data.profile_names.len();
                let _ = config::server::create_profile(format!("profile #{}", count), "{home}/oxideux/source", 49160, "0.0.0.0");
            },
            "r" => app_data.refresh_profile_names(),
            "c" => {
                let path = match config::config_dir_ext("oxideux") {
                    Ok(v) => v,
                    Err(e) => {
                        app_data.push_notice(e);
                        return;
                    }
                };

                #[cfg(target_os = "linux")]
                let command = "xdg-open";
                #[cfg(target_os = "windows")]
                let command = "explorer";

                match Command::new(command)
                    .arg(path)
                    .output() {
                        Ok(_) => (),
                        Err(e) => {
                            app_data.push_notice(e);
                        },
                    }
            },
            "q" => command.exit(),
            _ => unreachable!()
        },
        cli::OptionType::Error(e) => app_data.push_notice(e)
    }
}

fn state_manage_profile(app_data: &mut AppData, command: &mut app::Command) {
    app_data.refresh_cli();

    let profile = app_data.current_profile.as_ref().unwrap();
    
    // Error checking
    let mut errors = vec![];
    
    if let Err(e) = profile.parity_root.is_valid() {
        errors.push(format!("Parity root: {}.", e.to_string()));
    }

    if let Err(e) = profile.port.is_valid() {
        errors.push(format!("Port: {}.", e.to_string()));
    }
    
    if let Err(e) = profile.mask.is_valid() {
        errors.push(format!("Mask: {}.", e.to_string()));
    }

    if errors.len() != 0 {
        errors.push(format!("Due to {} previous error(s), the server may not be started.", errors.len()));
    }

    // Print our errors
    for error in &errors {
        cli::notice(error);
    }
    println!();

    // Display profile info
    cli::out(format!("Profile: {}", profile.name));
    cli::out(format!("Parity root: {}", profile.parity_root.get()));
    cli::out(format!("Port: {}", profile.port.get()));
    cli::out(format!("Mask: {}", profile.mask.get()));
    println!();

    let mut options = cli::InputOptions::new();

    if errors.len() == 0 {
        options.add_static("s", "Start server");
    }

    options
        .add_static("cn", "Change name")
        .add_static("cr", "Change parity root")
        .add_static("cp", "Change port")
        .add_static("cm", "Change mask")
        .add_static("erase", "Erase the profile (permanently)")
        .add_static("q", "Return");

    match options.get() {
        cli::OptionType::Dynamic(_) => unreachable!(),
        cli::OptionType::Static(key) => match key.as_ref() {
            "s" => command.queue_state("start_server"),
            "cn" => command.queue_state("change_name"),
            "cr" => command.queue_state("change_parity_root"),
            "cp" => command.queue_state("change_port"),
            "cm" => command.queue_state("change_mask"),
            "erase" => match config::server::erase_profile(&profile.name) {
                Ok(_) => {
                    match config::server::erase_profile(&profile.name) {
                        Ok(_) => command.queue_state("pick_profile"),
                        Err(e) => app_data.push_notice(e),
                    }
                },
                Err(e) => app_data.push_notice(format!("Error erasing file: {}", e)),
            }
            "q" => command.queue_state("pick_profile"),
            _ => unreachable!()
        },
        cli::OptionType::Error(e) => app_data.push_notice(e),
    }
}

fn state_change_name(app_data: &mut AppData, command: &mut app::Command) {
    app_data.refresh_cli();

    let profile = app_data.current_profile.as_mut().unwrap();

    cli::notice("Leave blank to cancel.");
    println!();

    cli::out(format!("Changing: name"));
    cli::out(format!("Current: {}", profile.name));

    let input = cli::input();
    if input.len() == 0 {
        command.queue_state("manage_profile");
        return;
    }

    match config::server::rename_profile(&profile.name, input.clone()) {
        Ok(_) => {
            profile.name = input;
            command.queue_state("manage_profile");
        },
        Err(e) => app_data.push_notice(e),
    }
}

macro_rules! state_change_property {
    ($fn_name:ident, $name:expr, $prop:ident, $intercept:expr) => {
        fn $fn_name(app_data: &mut AppData, command: &mut app::Command) {
            app_data.refresh_cli();

            let profile = app_data.current_profile.as_mut().unwrap();

            cli::notice("Leave blank to cancel.");
            println!();

            cli::out(format!("Changing: {}", $name));
            cli::out(format!("Current: {}", profile.$prop.get()));

            let input = cli::input();
            if input.len() == 0 {
                command.queue_state("manage_profile");
                return;
            }

            let parsed = match $intercept(input) {
                Ok(v) => v,
                Err(e) => {
                    app_data.push_notice(e);
                    return;
                }
            };

            match profile.$prop.safe_set(parsed) {
                Ok(_) => command.queue_state("save_updated_profile"),
                Err(e) => app_data.push_notice(e),
            }
        }
    };
}

state_change_property!(state_change_parity_root, "parity root", parity_root, |input| config::fill_path_placeholders(input) );
state_change_property!(state_change_port, "port", port, |input: String| input.parse::<u16>());
state_change_property!(state_change_mask, "mask", mask, |input| -> Result<String> { Result::Ok(input) });

fn state_save_updated_profile(app_data: &mut AppData, command: &mut app::Command) {
    app_data.refresh_cli();

    let profile = app_data.current_profile.as_mut().unwrap();

    cli::out(format!("Changes have been made to the following profile: {}", profile.name));
    cli::out("Would you like to save these changes?");
    println!();

    let mut options = cli::InputOptions::new();
    options
        .add_static("y", "Yes, save")
        .add_static("n", "No, do not save");

    match options.get() {
        cli::OptionType::Dynamic(_) => unreachable!(),
        cli::OptionType::Static(key) => match key.as_ref() {
            "y" => {
                if let Err(e) = config::server::save_profile(profile) {
                    app_data.push_notice(format!("Error saving profile: {}", e));
                } else {
                    app_data.push_notice("Profile successfully saved.");
                }
                command.queue_state("manage_profile");
            }
            "n" => command.queue_state("manage_profile"),
            _ => unreachable!()
        },
        cli::OptionType::Error(e) => app_data.push_notice(e),
    }
}

fn state_start_server(app_data: &mut AppData, command: &mut app::Command) {
    let profile = app_data.current_profile.as_ref().unwrap();
    let result = server(profile);
    app_data.push_notice(match result {
        Ok(_) => "Server terminated (OK)".to_string(),
        Err(e) => format!("Server terminated (ERROR): {}", e),
    });
    command.queue_state("manage_profile");
}

fn server(profile: &ServerProfile) -> Result<()> {
    let addr = format!("{}:{}", profile.mask.get(), profile.port.get());
    let listener = TcpListener::bind(&addr)?;

    println!(
        "Listening for connections on {}\nParity root: {}",
        addr,
        profile.parity_root.get()
    );

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                println!("Connection established: {:?}", stream.peer_addr());
                let result = handle_client(profile.clone(), &mut Connection(stream));
                println!("Connection terminated: {:?}", result);
            }
            Err(error) => {
                println!("Connection error: {}", error);
            }
        }
    }

    Ok(())
}

fn handle_client(profile: ServerProfile, conn: &mut Connection) -> Result<()> {
    let request = conn.read_request()?;

    match request {
        Request::Disconnect => {
            conn.shutdown(Shutdown::Both)?;
        }
        Request::GetFileCount => {
            let entries = parity::get_file_entries(PathBuf::from(profile.parity_root.get()))?;
            conn.send_request_result(RequestResult::Ok)?;
            conn.send_u32(entries.len() as u32)?;
        }
        Request::DownloadFileByIndex(index) => {
            let entries = parity::get_file_entries(PathBuf::from(profile.parity_root.get()))?;

            // // Index out of bounds
            if index as usize >= entries.len() {
                conn.send_request_result(RequestResult::ErrIndexOutOfBounds)?
                    .naturalize()?;
            }

            let entry = &entries[index as usize];
            conn.send_request_result(RequestResult::Ok)?;
            conn.send_string(&entry.name)?;
            conn.send_file(entry)?;
        }
        Request::DownloadFileByName(name) => {
            let parity_root = PathBuf::from(profile.parity_root.get());

            let mut file_path = parity_root.clone();
            file_path.push(name);
            file_path.canonicalize()?;

            // Unauthorized file access
            if !file_path.starts_with(parity_root) {
                conn.send_request_result(RequestResult::ErrUnauthorizedAccess)?
                    .naturalize()?;
            }

            let entry = parity::get_file_entry(file_path)?;
            conn.send_request_result(RequestResult::Ok)?;
            conn.send_file(&entry)?;
        }
        Request::DownloadAllFiles => {
            let entries = parity::get_file_entries(PathBuf::from(profile.parity_root.get()))?;
            conn.send_request_result(RequestResult::Ok)?;

            let count = entries.len();
            conn.send_u32(count as u32)?;

            for entry in entries {
                conn.send_string(&entry.name)?;
                conn.send_file(&entry)?;
                conn.read_request_result()?;
            }
        }
    }

    Ok(())
}
