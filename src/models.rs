use std::fmt::Display;

use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Default)]
pub struct OutboxMessages {
    pub uuid: String,
    pub payload: String,
    pub last_error: Option<String>,
    pub attempts: i32,
    pub exchange: String,
    pub routing_key: String,
    pub metadata: Option<String>,
    pub completed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Display for OutboxMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = format!(
            "OutboxMessages {{ uuid: {}, payload: {}, last_error: {:?}, attempts: {}, exchange: {}, routing_key: {}, metadata: {:?}, completed_at: {:?}, created_at: {:?}, updated_at: {:?} }}",
            self.uuid,
            self.payload,
            self.last_error,
            self.attempts,
            self.exchange,
            self.routing_key,
            self.metadata,
            self.completed_at,
            self.created_at,
            self.updated_at,
        );

        write!(f, "{}", &string)
    }
}
