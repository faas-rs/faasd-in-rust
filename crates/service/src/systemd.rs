type Err = Box<dyn std::error::Error + Send + Sync>;
use handlebars::Handlebars;
use std::{collections::HashMap, fs::File, io::Write, path::Path};

pub struct Systemd;

impl Systemd {
    pub fn enable(unit: String) -> Result<(), Err> {
        let output = std::process::Command::new("systemctl")
            .arg("enable")
            .arg(&unit)
            .output()?;
        if !output.status.success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to enable unit {}: {}",
                    unit,
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }
        Ok(())
    }

    pub fn start(unit: String) -> Result<(), Err> {
        let output = std::process::Command::new("systemctl")
            .arg("start")
            .arg(&unit)
            .output()?;
        if !output.status.success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to start unit {}: {}",
                    unit,
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }
        Ok(())
    }

    pub fn daemon_reload() -> Result<(), Err> {
        let output = std::process::Command::new("systemctl")
            .arg("daemon-reload")
            .output()?;
        if !output.status.success() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to reload systemd daemon: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }
        Ok(())
    }

    pub fn install_unit(name: String, tokens: HashMap<String, String>) -> Result<(), Err> {
        if tokens.get("Cwd").is_none_or(|v| v.is_empty()) {
            return Err("key Cwd expected in tokens parameter".into());
        }

        let tmpl_name = format!("./hack/{}.service", name);
        let mut handlebars = Handlebars::new();
        handlebars.register_template_file("template", &tmpl_name)?;

        let rendered = handlebars.render("template", &tokens)?;
        Self::write_unit(&format!("{}.service", name), rendered.as_bytes())?;
        Ok(())
    }

    pub fn write_unit(name: &str, content: &[u8]) -> Result<(), Err> {
        let path = Path::new("/lib/systemd/system").join(name);
        let mut file = File::create(path)?;
        file.write_all(content)?;
        Ok(())
    }
}
