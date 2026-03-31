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
    use crate::database::entities::lock::db_impl::LockKindSqlType;

    task_template_locks (id) {
        id -> Uuid,
        task_template_id -> Uuid,
        name_template -> Varchar,
        #[sql_name = "type"]
        lock_type -> LockKindSqlType,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    task_template_variables (id) {
        id -> Uuid,
        task_template_id -> Uuid,
        name -> Varchar,
        default_value -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    task_templates (id) {
        id -> Uuid,
        name -> Varchar,
        display_name_template -> Varchar,
        flake_ref_uri_template -> Varchar,
        flake_ref_args_template -> Array<Text>,
        features -> Array<Text>,
        group -> Nullable<Varchar>,
        created_at -> Timestamptz,
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
        created_at -> Timestamptz,
        claimed_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        last_heartbeat -> Nullable<Timestamptz>,
        group -> Nullable<Varchar>,
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

diesel::allow_tables_to_appear_in_same_query!(
    locks,
    task_template_locks,
    task_template_variables,
    task_templates,
    tasks,
    users,
);
