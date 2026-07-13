// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "LockKind_Type"))]
    pub struct LockKindType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "Role_Type"))]
    pub struct RoleType;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "TaskStatus_Type"))]
    pub struct TaskStatusType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::LockKindType;

    locks (id) {
        id -> Uuid,
        task_id -> Uuid,
        name -> Varchar,
        #[sql_name = "type"]
        type_ -> LockKindType,
        poisoned_by_task -> Nullable<Uuid>,
    }
}

diesel::table! {
    task_groups (id) {
        id -> Uuid,
        name -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TaskStatusType;

    tasks (id) {
        id -> Uuid,
        display_name -> Varchar,
        status -> TaskStatusType,
        features -> Array<Nullable<Text>>,
        flake_ref_uri -> Varchar,
        flake_ref_args -> Array<Nullable<Text>>,
        created_at -> Timestamptz,
        finished_at -> Nullable<Timestamptz>,
        claimed_at -> Nullable<Timestamptz>,
        last_heartbeat -> Nullable<Timestamptz>,
        group_id -> Uuid,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::RoleType;

    users (id) {
        id -> Uuid,
        name -> Varchar,
        role -> RoleType,
    }
}

diesel::joinable!(tasks -> task_groups (group_id));

diesel::allow_tables_to_appear_in_same_query!(locks, task_groups, tasks, users,);
