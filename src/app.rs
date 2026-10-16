use crate::container::Container;
use crate::docker;
use crate::widget::Widgets;

pub struct App {
    pub containers: Vec<Container>,
    pub active_container: usize,
    pub active_widget: Widgets,
    pub scroll_offset: usize,
}

impl App {
    pub async fn new() -> Self {
        let mut app = Self {
            containers: Vec::new(),
            active_container: 0,
            active_widget: Widgets::Containers,
            scroll_offset: 0,
        };

        app.get_containers().await;
        app.refresh_logs().await;

        app
    }

    pub async fn get_containers(&mut self) {
        self.containers = docker::fetch_containers().await;
    }

    pub async fn refresh_logs(&mut self) {
        if self.containers.is_empty() {
            return;
        }

        let id = self.current_container().id.clone();
        let logs = docker::fetch_logs(&id).await;
        self.current_container_mut().logs = logs;
    }

    pub fn current_container(&self) -> &Container {
        &self.containers[self.active_container]
    }

    pub fn current_container_mut(&mut self) -> &mut Container {
        &mut self.containers[self.active_container]
    }

    async fn select_container(&mut self, idx: usize) {
        self.active_container = idx;
        self.scroll_offset = 0;
        self.refresh_logs().await;
    }

    pub async fn next_container(&mut self) {
        if !self.containers.is_empty() {
            let idx = (self.active_container + 1) % self.containers.len();
            self.select_container(idx).await;
        }
    }

    pub async fn previous_container(&mut self) {
        if !self.containers.is_empty() {
            let idx = self
                .active_container
                .checked_sub(1)
                .unwrap_or(self.containers.len() - 1);
            self.select_container(idx).await;
        }
    }

    pub async fn set_container(&mut self, id: usize) {
        if id < self.containers.len() {
            self.select_container(id).await;
        }
    }

    pub async fn refresh(&mut self) -> std::io::Result<()> {
        let old_id = self
            .containers
            .get(self.active_container)
            .map(|c| c.id.clone());

        self.get_containers().await;

        if self.containers.is_empty() {
            self.active_container = 0;
            return Ok(());
        }

        // Stay on the same container if it still exists, otherwise clamp the index
        self.active_container = old_id
            .and_then(|id| self.containers.iter().position(|c| c.id == id))
            .unwrap_or(self.active_container.min(self.containers.len() - 1));

        self.refresh_logs().await;

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
