use crate::schema::feedbacks;
//use diesel::sql_types::Timestamp;
use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable)]
#[diesel(table_name = feedbacks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Feedback {
    pub id: i32,
    pub email: String,
    pub feedback: String,
    //pub created_at: Timestamp,
}

#[derive(Insertable)]
#[diesel(table_name = feedbacks)]
pub struct NewFeedback<'a> {
    pub email: &'a str,
    pub feedback: &'a str,
}
