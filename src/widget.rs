use strum::{EnumCount, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount, EnumIter)]
pub enum Widgets {
    Containers,
    TTY,
    Logs,
}

impl Widgets {
    pub fn next(self) -> Self {
        let variants: Vec<Widgets> = Widgets::iter().collect();
        let idx = variants.iter().position(|w| *w == self).unwrap();
        variants[(idx + 1) % variants.len()]
    }

    pub fn prev(self) -> Self {
        let variants: Vec<Widgets> = Widgets::iter().collect();
        let idx = variants.iter().position(|w| *w == self).unwrap();
        variants[(idx + variants.len() - 1) % variants.len()]
    }
}
