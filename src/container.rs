use std::process::Command;

pub struct Container {
    pub id: String,
    pub image: String,
    pub status: String,
    pub name: String,
    pub logs: String,
}

impl Container {
    pub fn new(line: &str) -> Self {
        let mut fields = line.split('|');

        Self {
            id: fields.next().unwrap_or("").trim().into(),
            image: fields.next().unwrap_or("").trim().into(),
            status: fields.next().unwrap_or("").trim().into(),
            name: fields.next().unwrap_or("").trim().into(),
            logs: String::new(),
        }
    }

    pub fn get_logs(&mut self) -> String {
        let output = Command::new("docker")
            .args(["logs", "--tail", "200", &self.id])
            .output()
            .expect("Failed to execute docker logs command");

        self.logs = String::from_utf8_lossy(&output.stdout).to_string();

        self.logs.clone()
    }

    pub fn stop_container(&self) {
        let _ = Command::new("docker")
            .args(["stop", &self.id])
            .output()
            .expect("Failed to execute docker stop command");
    }

    pub fn start_container(&self) {
        let _ = Command::new("docker")
            .args(["start", &self.id])
            .output()
            .expect("Failed to execute docker start command");
    }

    pub fn restart_container(&self) {
        let _ = Command::new("docker")
            .args(["restart", &self.id])
            .output()
            .expect("Failed to execute docker restart command");
    }
}
