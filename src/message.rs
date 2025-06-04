#[allow(dead_code)]
#[derive(Clone)]
pub struct Message {
    pub timestamp: String, // Timestamp in the format `YY-MM-DD HH:MM:SS`
    pub content: String,   // The actual message content
}
