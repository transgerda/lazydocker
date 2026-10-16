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
}
