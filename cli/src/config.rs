use std::path::PathBuf;

use crate::repl_reader::Theme;

pub struct Config {
    pub theme: Theme,
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("sgleam");
        p.push("config");
        p
    })
}

pub fn load() -> Config {
    let mut config = Config { theme: Theme::Dark };
    let Some(path) = config_path() else {
        return config;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return config;
    };
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == "theme"
            && let Some(theme) = Theme::parse(value.trim())
        {
            config.theme = theme;
        }
    }
    config
}

pub fn save(theme: Theme) {
    let Some(path) = config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, format!("theme={}\n", theme.name()));
}
