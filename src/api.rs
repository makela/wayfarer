use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use std::env;

use super::model::{Feedback, NewFeedback};

pub fn get_connection_pool() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(url);
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool")
}

pub async fn create_feedback(conn: &mut PgConnection, email: &str, feedback: &str) -> Feedback {
    use crate::schema::feedbacks;

    let new_feedback = NewFeedback { email, feedback };

    diesel::insert_into(feedbacks::table)
        .values(&new_feedback)
        .returning(Feedback::as_returning())
        .get_result(conn)
        .expect("Error saving new post")
}
