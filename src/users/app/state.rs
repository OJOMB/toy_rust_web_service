use crate::users::service::idos;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid;

#[derive(Clone)]
pub struct State {
    pub users: Arc<Mutex<HashMap<uuid::Uuid, idos::User>>>,
}

impl State {
    pub fn new() -> Self {
        let u = idos::User::new_dummy();

        let mut users: std::collections::HashMap<uuid::Uuid, idos::User> =
            std::collections::HashMap::new();

        // add some dummy users for testing
        users.insert(u.id, u);

        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}
