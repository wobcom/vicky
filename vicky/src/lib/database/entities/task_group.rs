use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskGroup {
    pub id: Uuid,
    pub name: String,

    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

impl TaskGroup {
    fn new<S: Into<String>>(name: S) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: Utc::now()
        }
    }
}

pub mod db_impl {
    use chrono::{DateTime, Utc};
    use diesel::prelude::*;
    use serde::Serialize;
    use uuid::Uuid;

    use crate::database::entities::task_group::{TaskGroup};
    use crate::database::schema::task_groups;
    use crate::errors::VickyError;
    use crate::query::FilterParams;

    #[derive(Selectable, Identifiable, Queryable, Debug, Serialize)]
    #[diesel(table_name = task_groups)]
    pub struct DbTaskGroup {
        pub id: Uuid,
        pub name: String,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Insertable, Debug)]
    #[diesel(table_name = task_groups)]
    pub struct NewDbTaskGroup {
        pub name: String,
    }

    impl From<DbTaskGroup> for TaskGroup {
        fn from(task_group: DbTaskGroup) -> TaskGroup {
            TaskGroup {
                id: task_group.id,
                name: task_group.name,
                created_at: Utc::now(),
            }
        }
    }

    pub trait TaskGroupDatabase {
        fn get_task_groups(&mut self) -> Result<Vec<TaskGroup>, VickyError>;
        fn get_task_groups_filtered<F: Into<FilterParams>>(
            &mut self,
            filters: F,
        ) -> Result<Vec<TaskGroup>, VickyError>;
        fn count_all_task_groups(&mut self) -> Result<i64, VickyError>;
    }

    impl TaskGroupDatabase for PgConnection {
        fn count_all_task_groups(&mut self) -> Result<i64, VickyError> {
            let tasks_count_b = task_groups::table.into_boxed();
            let tasks_count: i64 = tasks_count_b.count().first(self)?;

            Ok(tasks_count)
        }

        fn get_task_groups_filtered<F: Into<FilterParams>>(
            &mut self,
            filters: F,
        ) -> Result<Vec<TaskGroup>, VickyError> {
            let filters = filters.into();

            let mut db_tasks_build = task_groups::table.into_boxed();

            if let Some(r_limit) = filters.limit {
                db_tasks_build = db_tasks_build.limit(r_limit)
            }
            if let Some(r_offset) = filters.offset {
                db_tasks_build = db_tasks_build.offset(r_offset)
            }
            let db_task_groups = db_tasks_build
                .order(task_groups::created_at.desc())
                .load::<DbTaskGroup>(self)?;

            let task_groups: Vec<TaskGroup> =
                db_task_groups.into_iter().map(|t| t.into()).collect();

            Ok(task_groups)
        }

        fn get_task_groups(&mut self) -> Result<Vec<TaskGroup>, VickyError> {
            self.get_task_groups_filtered(None)
        }
    }
}
