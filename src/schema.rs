// @generated automatically by Diesel CLI.

diesel::table! {
    feedbacks (id) {
        id -> Int4,
        email -> Varchar,
        feedback -> Text,
        created_at -> Timestamp,
    }
}
