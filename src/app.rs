use crate::container::Container;
use crate::widget::Widgets;
use std::process::Command;

pub struct App {
    pub containers: Vec<Container>,
    pub active_container: usize,
    pub active_widget: Widgets,
    pub scroll_offset: usize,
    // plus your logs fields: container_logs, logs_scroll_offset, ...
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            containers: Vec::new(),
            active_container: 0,
            active_widget: Widgets::Containers,
            scroll_offset: 0,
        };

        app.get_containers();

        if !app.containers.is_empty() {
            app.current_container_mut().get_logs();
        }

        app
    }

    pub fn get_containers(&mut self) {
        let Ok(output) = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--format",
                "{{.ID}}|{{.Image}}|{{.Status}}|{{.Names}}",
            ])
            .output()
        else {
            return;
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut containers: Vec<Container> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(Container::new)
            .collect();

        containers.sort_by_key(|c| (!c.status.starts_with("Up"), c.status.clone()));

        self.containers = containers;
    }

    pub fn current_container(&self) -> &Container {
        &self.containers[self.active_container]
    }

    pub fn current_container_mut(&mut self) -> &mut Container {
        &mut self.containers[self.active_container]
    }

    fn select_container(&mut self, idx: usize) {
        self.active_container = idx;
        self.scroll_offset = 0;
        self.current_container_mut().get_logs();
    }

    pub fn next_container(&mut self) {
        if !self.containers.is_empty() {
            let idx = (self.active_container + 1) % self.containers.len();
            self.select_container(idx);
        }
    }

    pub fn previous_container(&mut self) {
        if !self.containers.is_empty() {
            let idx = self
                .active_container
                .checked_sub(1)
                .unwrap_or(self.containers.len() - 1);
            self.select_container(idx);
        }
    }

    pub fn set_container(&mut self, id: usize) {
        if id < self.containers.len() {
            self.select_container(id);
        }
    }

    pub fn refresh(&mut self) -> std::io::Result<()> {
        let old_id = self
            .containers
            .get(self.active_container)
            .map(|c| c.id.clone());

        self.get_containers();

        if self.containers.is_empty() {
            self.active_container = 0;
            return Ok(());
        }

        // Stay on the same container if it still exists, otherwise clamp the index
        self.active_container = old_id
            .and_then(|id| self.containers.iter().position(|c| c.id == id))
            .unwrap_or(self.active_container.min(self.containers.len() - 1));

        self.current_container_mut().get_logs();

        Ok(())
    }

    pub fn scroll_down_logs(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_up_logs(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines).min(
            self.current_container()
                .logs
                .lines()
                .count()
                .saturating_sub(1),
        );
    }
}
