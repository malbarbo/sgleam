use std::path::PathBuf;

pub struct Config {
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: "dark".into(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("sgleam");
        p.push("config");
        p
    })
}

pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };
    let mut config = Config::default();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == "theme"
        {
            config.theme = value.trim().to_string();
        }
    }
    config
}

pub fn save(theme: &str) {
    let path = match config_path() {
        Some(p) => p,
        None => return,
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, format!("theme={theme}\n"));
}
