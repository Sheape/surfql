pub struct Crawler {
    url: String,
}

impl Crawler {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}
