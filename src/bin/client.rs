use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;

use oxideux_rs::app;
use oxideux_rs::cli;
use oxideux_rs::config::{self, ClientProfile};
use oxideux_rs::connection::Connection;
use oxideux_rs::request::{Request, RequestResult};
use oxideux_rs::validated_values::ValidatedValue;

use anyhow::{self, Result};

#[derive(Default)]
struct AppData {
    profile_names: Vec<String>,
    current_profile: Option<ClientProfile>,
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
        self.profile_names = config::client::get_profile_names().expect("Something went wrong when refreshing profile names");
    }
}

fn main() -> Result<()> {
    config::client::init_config_file()?;

    let app_data = AppData::default();

    let mut app = app::App::new(app_data);
    app.register_state("pick_profile", state_pick_profile);
    app.register_state("manage_profile", state_manage_profile);
    app.register_state("change_name", state_change_name);
    app.register_state("change_parity_root", state_change_parity_root);
    app.register_state("change_port", state_change_port);
    app.register_state("change_ipv4", state_change_ipv4);
    app.register_state("save_updated_profile", state_save_updated_profile);
    app.register_state("start_client", state_start_client);

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
            let profile = config::client::get_profile(profile_name).unwrap();
            app_data.current_profile = Some(profile);
            command.queue_state("manage_profile");
        },
        cli::OptionType::Static(key) => match key.as_str() {
            "a" => {
                let count = app_data.profile_names.len();
                let _ = config::client::create_profile(format!("profile #{}", count), "{download}", 49160, "localhost");
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
    
    if let Err(e) = profile.ipv4.is_valid() {
        errors.push(format!("IPv4: {}.", e.to_string()));
    }

    if errors.len() != 0 {
        errors.push(format!("Due to {} previous error(s), the client may not be started.", errors.len()));
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
    cli::out(format!("IPv4: {}", profile.ipv4.get()));
    println!();

    let mut options = cli::InputOptions::new();

    if errors.len() == 0 {
        options.add_static("s", "Start client");
    }

    options
        .add_static("cn", "Change name")
        .add_static("cr", "Change parity root")
        .add_static("cp", "Change port")
        .add_static("ci", "Change IPv4")
        .add_static("erase", "Erase the profile (permanently)")
        .add_static("q", "Return");

    match options.get() {
        cli::OptionType::Dynamic(_) => unreachable!(),
        cli::OptionType::Static(key) => match key.as_ref() {
            "s" => command.queue_state("start_client"),
            "cn" => command.queue_state("change_name"),
            "cr" => command.queue_state("change_parity_root"),
            "cp" => command.queue_state("change_port"),
            "ci" => command.queue_state("change_ipv4"),
            "erase" => match config::client::erase_profile(&profile.name) {
                Ok(_) => {
                    match config::client::erase_profile(&profile.name) {
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

    match config::client::rename_profile(&profile.name, input.clone()) {
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
state_change_property!(state_change_ipv4, "ipv4", ipv4, |input| -> Result<String> { Result::Ok(input) });

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
                if let Err(e) = config::client::save_profile(profile) {
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

fn state_start_client(app_data: &mut AppData, command: &mut app::Command) {
    let profile = app_data.current_profile.as_ref().unwrap();
    let result = client(profile);
    app_data.push_notice(match result {
        Ok(_) => "Client terminated (OK)".to_string(),
        Err(e) => format!("Client terminated (ERROR): {}", e),
    });
    command.queue_state("manage_profile");
}

fn client(profile: &ClientProfile) -> Result<()> {
    let addr = format!(
        "{}:{}",
        profile.ipv4.get(),
        profile.port.get()
    );
    let stream = TcpStream::connect(&addr)?;

    println!(
        "Established connection to {}\nParity root: {}",
        addr,
        profile.parity_root.get()
    );

    let mut conn = Connection(stream);

    let request = Request::DownloadAllFiles;
    conn.send_request(&request)?;

    match request {
        Request::Disconnect => {}
        Request::GetFileCount => {
            conn.read_request_result()?;
            let count = conn.read_u32()?;
            println!("There are {} files", count);
        }
        Request::DownloadFileByIndex(_) => {
            conn.read_request_result()?;
            let name = conn.read_string()?;
            let mut output = PathBuf::from(profile.parity_root.get());
            output.push(name);
            conn.read_file(&output)?;
        }
        Request::DownloadFileByName(name) => {
            conn.read_request_result()?;
            let mut output = PathBuf::from(profile.parity_root.get());
            output.push(name);
            conn.read_file(&output)?;
        }
        Request::DownloadAllFiles => {
            conn.read_request_result()?;
            let count = conn.read_u32()?;
            for i in 0..count {
                println!();
                let name = conn.read_string()?;
                let mut output = PathBuf::from(profile.parity_root.get());
                println!("({}/{}) Destination file: {:?}/{}", i, count - 1, &output, name);
                output.push(name);
                conn.read_file(&output)?;
                conn.send_request_result(RequestResult::Ok)?;
            }
        }
    }

    Ok(())
}
