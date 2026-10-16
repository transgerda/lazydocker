use crate::container::Container;
use tokio::process::Command;

pub async fn fetch_containers() -> Vec<Container> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--format",
            "{{.ID}}|{{.Image}}|{{.Status}}|{{.Names}}",
        ])
        .output()
        .await
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut containers: Vec<Container> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(Container::new)
        .collect();

    containers.sort_by_key(|c| (!c.status.starts_with("Up"), c.status.clone()));
    containers
}

pub async fn fetch_logs(id: &str) -> String {
    let output = Command::new("docker")
        .args(["logs", "--tail", "200", id])
        .output()
        .await
        .unwrap();

    String::from_utf8_lossy(&output.stdout).to_string()
}

pub async fn stop(id: &str) {
    let _ = Command::new("docker").args(["stop", id]).output().await;
}

pub async fn start(id: &str) {
    let _ = Command::new("docker").args(["start", id]).output().await;
}

pub async fn restart(id: &str) {
    let _ = Command::new("docker").args(["restart", id]).output().await;
}
