// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::database::entities::lock::db_impl::LockKindSqlType;

    locks (id) {
        id -> Uuid,
        task_id -> Uuid,
        name -> Varchar,
        #[sql_name = "type"]
        lock_type -> LockKindSqlType,
        poisoned_by_task -> Nullable<Uuid>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::database::entities::task::db_impl::TaskStatusSqlType;

    tasks (id) {
        id -> Uuid,
        display_name -> Varchar,
        status -> TaskStatusSqlType,
        features -> Array<Text>,
        flake_ref_uri -> Varchar,
        flake_ref_args -> Array<Text>,
        created_at -> Timestamp,
        claimed_at -> Nullable<Timestamp>,
        finished_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::database::entities::user::db_impl::RoleSqlType;

    users (id) {
        id -> Uuid,
        name -> Varchar,
        role -> RoleSqlType,
    }
}

diesel::allow_tables_to_appear_in_same_query!(locks, tasks, users,);
