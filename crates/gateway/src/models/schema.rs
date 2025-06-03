// @generated automatically by Diesel CLI.

diesel::table! {
    users (uid) {
        uid -> Uuid,
        username -> Varchar,
        password_hash -> Varchar,
        created_at -> Timestamp,
    }
}
