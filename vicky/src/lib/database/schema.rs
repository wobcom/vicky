// @generated automatically by Diesel CLI.

diesel::table! {
    locks (id) {
        id -> Int4,
        task_id -> Nullable<Uuid>,
        name -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
    }
}

diesel::table! {
    tasks (id) {
        id -> Uuid,
        display_name -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        flake_ref_uri -> Nullable<Varchar>,
        flake_ref_args -> Nullable<Varchar>,
    }
}

diesel::joinable!(locks -> tasks (task_id));

diesel::allow_tables_to_appear_in_same_query!(
    locks,
    tasks,
);
