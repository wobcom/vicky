// @generated automatically by Diesel CLI.

diesel::table! {
    locks (id) {
        id -> Nullable<Uuid>,
        task_id -> Uuid,
        name -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
    }
}

diesel::table! {
    tasks (id) {
        id -> Uuid,
        display_name -> Varchar,
        status -> Varchar,
        features -> Array<Nullable<Text>>,
        flake_ref_uri -> Varchar,
        flake_ref_args -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    users (sub) {
        sub -> Uuid,
        name -> Varchar,
        role -> Varchar,
    }
}

diesel::joinable!(locks -> tasks (task_id));

diesel::allow_tables_to_appear_in_same_query!(
    locks,
    tasks,
    users,
);
