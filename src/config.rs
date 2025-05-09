use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::validated_values::*;
use anyhow::{anyhow, Result};
use directories::{BaseDirs, UserDirs};

#[derive(Debug, Clone)]
pub struct ServerProfile {
    pub name: String,
    pub parity_root: ValidatedDirectory,
    pub port: ValidatedPort,
    pub mask: ValidatedIPv4,
}

#[derive(Debug, Clone)]
pub struct ClientProfile {
    pub name: String,
    pub parity_root: ValidatedDirectory,
    pub port: ValidatedPort,
    pub ipv4: ValidatedIPv4,
}

#[inline]
fn appdata_dir() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or(anyhow!("Home directory could not be retrieved."))?
        .data_local_dir()
        .to_path_buf())
}

#[inline]
fn download_dir() -> Result<PathBuf> {
    Ok(UserDirs::new()
        .ok_or(anyhow!("Home directory could not be retrieved."))?
        .download_dir()
        .ok_or(anyhow!("Download directory could not be retrieved."))?
        .to_path_buf())
}

#[inline]
fn home_dir() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or(anyhow!("Home directory could not be retrieved."))?
        .home_dir()
        .to_path_buf())
}

#[inline]
pub fn config_dir() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or(anyhow!("Home directory could not be retrieved."))?
        .config_local_dir()
        .to_path_buf())
}

#[inline]
pub fn config_dir_ext<S: AsRef<str>>(ext: S) -> Result<PathBuf> {
    let mut path = config_dir()?;
    path.push(ext.as_ref());
    Ok(path)
}

struct PathPlaceholderReplacer(String);

impl PathPlaceholderReplacer {
    fn placeholder<S: AsRef<str>>(&mut self, replace: S, with: PathBuf) {
        if self.0.starts_with(replace.as_ref()) {
            self.0 = self.0.replacen(replace.as_ref(), &with.to_string_lossy().to_string(), 1);
        }
    }
}

#[inline]
pub fn fill_path_placeholders(string_path: String) -> Result<String> {
    let mut ppr = PathPlaceholderReplacer(string_path);
    ppr.placeholder("~", home_dir()?);
    ppr.placeholder("{home}", home_dir()?);
    ppr.placeholder("{config}", config_dir()?);
    ppr.placeholder("{appdata}", appdata_dir()?);
    ppr.placeholder("{download}", download_dir()?);
    Ok(ppr.0)
}



pub(self) mod json_help {
    use super::*;
    use json::object::Object;
    use json::JsonValue;

    pub fn config_root_object<S: AsRef<str>>(ext: S) -> Result<json::object::Object> {
        use super::config_dir_ext;
        use super::fs;

        let path = config_dir_ext(ext)?;
        let source = fs::read_to_string(&path)?;

        let data = json::parse(&source)?;
        if let JsonValue::Object(o) = data {
            return Ok(o);
        }
        Err(anyhow!("Could not get config root object"))
    }

    #[inline]
    fn get_object_key<S: AsRef<str>>(object: &Object, key: S) -> Result<&JsonValue> {
        object.get(key.as_ref()).ok_or(anyhow!(format!(
            "'{}' key was not found in object {:?}",
            key.as_ref(),
            object
        )))
    }

    #[inline]
    fn get_mut_object_key<S: AsRef<str>>(object: &mut Object, key: S) -> Result<&mut JsonValue> {
        object.get_mut(key.as_ref()).ok_or(anyhow!(format!(
            "'{}' key was not found in mutable object",
            key.as_ref()
        )))
    }

    macro_rules! object_get {
        ($name:ident, $vtype:ident) => {
            #[inline]
            pub fn $name<S: AsRef<str>>(object: &Object, key: S) -> Result<&$vtype> {
                if let JsonValue::$vtype(inner) = get_object_key(object, &key)? {
                    return Ok(inner);
                }
                Err(anyhow!(format!(
                    "Expected key '{}' to be of type {}.",
                    key.as_ref(),
                    stringify!($vtype)
                )))
            }
        };
    }

    object_get!(object_get_object, Object);


    pub fn object_get_mut_object<S: AsRef<str>>(object: &mut Object, key: S) -> Result<&mut Object> {
        if let JsonValue::Object(inner) = get_mut_object_key(object, &key)? {
            return Ok(inner);
        }
        Err(anyhow!(format!(
            "Expected key '{}' to be of type Object.",
            key.as_ref()
        )))
    }

    #[inline]
    pub fn object_get_u16<S: AsRef<str>>(object: &Object, key: S) -> Result<u16> {
        let value = get_object_key(object, key)?;
        Ok(value
            .as_u16()
            .ok_or(anyhow!("Could not interpret value as u16"))?)
    }

    #[inline]
    pub fn object_get_str<S: AsRef<str>>(object: &Object, key: S) -> Result<&str> {
        let value = get_object_key(object, key)?;
        Ok(value
            .as_str()
            .ok_or(anyhow!("Could not interpret value as u16"))?)
    }
}

pub(self) mod common {
    use std::fs::OpenOptions;

    use super::*;

    /// Initializes a config file if it does not already exist.
    /// Returns true if an initialization occured, false otherwise.
    pub fn init_config_file<S: AsRef<str>>(ext: S, default_data: &'static [u8]) -> Result<bool> {
        let config_file = config_dir_ext(ext)?;
        let initialize = !config_file.exists();
        if initialize {
            let _ = fs::create_dir_all(config_file.parent().ok_or(anyhow!(format!(
                "Couldn't initialize path: {:?}",
                config_file.parent()
            )))?)?;
            let default_config = default_data;
            let mut file = File::create(config_file)?;
            let _ = file.write(default_config)?;
        }
        Ok(initialize)
    }

    pub fn overwrite_config_file<S: AsRef<str>>(ext: S, data: &[u8]) -> Result<()> {
        let config_file_path = config_dir_ext(ext)?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(config_file_path)?;
        file.write(data)?;
        Ok(())
    }

    pub fn get_profile_names<S: AsRef<str>>(ext: S) -> Result<Vec<String>> {
        let mut profile_names = vec![];

        let root = json_help::config_root_object(ext)?;
        let profiles = json_help::object_get_object(&root, "profiles")?;

        for (key, _) in profiles.iter() {
            if key.len() == 0 {
                continue;
            }
            profile_names.push(key.into());
        }

        Ok(profile_names)
    }

    pub fn erase_profile<S: AsRef<str>, T: AsRef<str>>(ext: S, profile_name: T) -> Result<()> {
        let mut root = json_help::config_root_object(ext.as_ref())?;
        let profiles = json_help::object_get_mut_object(&mut root, "profiles")?;
        profiles.remove(profile_name.as_ref());
        overwrite_config_file(ext, root.dump().as_bytes())?;
        Ok(())
    }

    pub fn rename_profile<S: AsRef<str>, T: ToString, V: AsRef<str>>(ext: S, profile_name: T, new_name: V) -> Result<()> {
        let mut root = json_help::config_root_object(ext.as_ref())?;
        let profiles = json_help::object_get_mut_object(&mut root, "profiles")?;
        if let Some(_) = profiles.get(new_name.as_ref()) {
            return Err(anyhow!(format!("Profile '{}' already exists", new_name.as_ref())));
        }
        let profile = json_help::object_get_object(&profiles, profile_name.to_string().clone())?.clone();
        profiles.insert(new_name.as_ref(), json::JsonValue::Object(profile));
        profiles.remove(&profile_name.to_string());
        overwrite_config_file(ext, root.dump().as_bytes())?;
        Ok(())
    }

    pub fn get_profile_object<S: AsRef<str>, T: AsRef<str>>(
        ext: S,
        profile_name: T,
    ) -> Result<json::object::Object> {
        let root = json_help::config_root_object(ext)?;
        let profiles = json_help::object_get_object(&root, "profiles")?;
        let profile = json_help::object_get_object(&profiles, profile_name.as_ref())?;
        Ok(profile.clone())
    }
}

pub mod server {
    use super::*;

    #[inline]
    fn config_ext() -> &'static str {
        "oxideux/server_config.json"
    }

    #[inline]
    pub fn init_config_file() -> Result<()> {
        if common::init_config_file(
            config_ext(),
            include_bytes!("../static_res/default_server_config.json"),
        )? {
            create_profile("default", "{home}/oxideux/source", 49160, "0.0.0.0")?;
        }
        Ok(())
    }

    #[inline]
    pub fn get_profile_names() -> Result<Vec<String>> {
        common::get_profile_names(config_ext())
    }

    pub fn get_profile<S: AsRef<str>>(profile_name: S) -> Result<ServerProfile> {
        let profile_object =
            common::get_profile_object(config_ext(), profile_name.as_ref())?;

        let path = fill_path_placeholders(
            json_help::object_get_str(&profile_object, "parity_root")?.to_string(),
        )?;

        let parity_root = ValidatedDirectory::new(path);
        let port = ValidatedPort::new(json_help::object_get_u16(&profile_object, "port")?);
        let mask = ValidatedIPv4::new(json_help::object_get_str(&profile_object, "mask")?.into());

        let profile = ServerProfile {
            name: profile_name.as_ref().to_string(),
            parity_root,
            port,
            mask,
        };
        Ok(profile)
    }

    pub fn save_profile(profile: &ServerProfile) -> Result<()> {
        let mut root = json_help::config_root_object(config_ext())?;
        let profiles = json_help::object_get_mut_object(&mut root, "profiles")?;
        let data = json::object! {
            "parity_root": json::JsonValue::String(profile.parity_root.get().clone()),
            "port": json::JsonValue::Number(json::number::Number::from(*profile.port.get())),
            "mask": json::JsonValue::String(profile.mask.get().clone()),
        };
        profiles.insert(&profile.name, data);
        common::overwrite_config_file(config_ext(), root.dump().as_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn erase_profile<S: AsRef<str>>(profile_name: S) -> Result<()> {
        common::erase_profile(config_ext(), profile_name)
    }

    pub fn create_profile<S: ToString, T: ToString, V: ToString>(profile_name: S, parity_root: T, port: u16, mask: V) -> Result<()> {
        let profile = ServerProfile {
            name: profile_name.to_string(),
            parity_root: ValidatedDirectory::new(parity_root.to_string()),
            port: ValidatedPort::new(port),
            mask: ValidatedIPv4::new(mask.to_string()),
        };
        save_profile(&profile)
    }

    #[inline]
    pub fn rename_profile<S: ToString, T: AsRef<str>>(profile_name: S, new_name: T) -> Result<()> {
        common::rename_profile(config_ext(), profile_name, new_name)
    }
}

pub mod client {
    use super::*;

    #[inline]
    fn config_ext() -> &'static str {
        "oxideux/client_config.json"
    }

    #[inline]
    pub fn init_config_file() -> Result<()> {
        if common::init_config_file(
            config_ext(),
            include_bytes!("../static_res/default_client_config.json"),
        )? {
            create_profile("default", "{download}", 49160, "localhost")?;
        }
        Ok(())
    }

    #[inline]
    pub fn get_profile_names() -> Result<Vec<String>> {
        common::get_profile_names(config_ext())
    }

    pub fn get_profile<S: AsRef<str>>(profile_name: S) -> Result<ClientProfile> {
        let profile_object =
            common::get_profile_object(config_ext(), profile_name.as_ref())?;

        let path = fill_path_placeholders(
            json_help::object_get_str(&profile_object, "parity_root")?.to_string(),
        )?;

        let parity_root = ValidatedDirectory::new(path);
        let port = ValidatedPort::new(json_help::object_get_u16(&profile_object, "port")?);
        let ip = ValidatedIPv4::new(json_help::object_get_str(&profile_object, "ipv4")?.into());

        let profile = ClientProfile {
            name: profile_name.as_ref().to_string(),
            parity_root,
            port,
            ipv4: ip,
        };
        Ok(profile)
    }

    pub fn save_profile(profile: &ClientProfile) -> Result<()> {
        let mut root = json_help::config_root_object(config_ext())?;
        let profiles = json_help::object_get_mut_object(&mut root, "profiles")?;
        let data = json::object! {
            "parity_root": json::JsonValue::String(profile.parity_root.get().clone()),
            "port": json::JsonValue::Number(json::number::Number::from(*profile.port.get())),
            "ipv4": json::JsonValue::String(profile.ipv4.get().clone()),
        };
        profiles.insert(&profile.name, data);
        common::overwrite_config_file(config_ext(), root.dump().as_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn erase_profile<S: AsRef<str>>(profile_name: S) -> Result<()> {
        common::erase_profile(config_ext(), profile_name)
    }

    pub fn create_profile<S: ToString, T: ToString, V: ToString>(profile_name: S, parity_root: T, port: u16, ipv4: V) -> Result<()> {
        let profile = ClientProfile {
            name: profile_name.to_string(),
            parity_root: ValidatedDirectory::new(parity_root.to_string()),
            port: ValidatedPort::new(port),
            ipv4: ValidatedIPv4::new(ipv4.to_string()),
        };
        save_profile(&profile)
    }

    #[inline]
    pub fn rename_profile<S: ToString, T: AsRef<str>>(profile_name: S, new_name: T) -> Result<()> {
        common::rename_profile(config_ext(), profile_name, new_name)
    }
}
