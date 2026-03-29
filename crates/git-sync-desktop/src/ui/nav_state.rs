#[derive(Clone, Debug)]
pub enum NavRequest {
    OpenSettings(Option<usize>),
    Back,
}

pub struct NavState {
    pub request: Option<NavRequest>,
}

impl NavState {
    pub fn new() -> Self {
        Self { request: None }
    }
}
