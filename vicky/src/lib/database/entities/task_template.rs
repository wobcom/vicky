use crate::database::entities::lock::{Lock, LockKind};
use crate::database::entities::task::{FlakeRef, Task, TaskStatus};
use crate::database::entities::task_template::db_impl::{
    DbTaskTemplate, DbTaskTemplateLock, DbTaskTemplateVariable,
};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTemplateVariable {
    pub name: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTemplateLock {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: LockKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: Uuid,
    pub name: String,
    pub display_name_template: String,
    pub flake_ref: FlakeRef,
    pub locks: Vec<TaskTemplateLock>,
    pub features: Vec<String>,
    pub group: Option<String>,
    pub variables: Vec<TaskTemplateVariable>,

    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TaskTemplateError {
    #[error("template name must not be empty")]
    EmptyName,

    #[error("template variable name must not be empty")]
    EmptyVariableName,

    #[error("duplicate template variable: {0}")]
    DuplicateVariable(String),

    #[error("unclosed variable symbol in template value")]
    UnclosedVariableMarker,

    #[error("empty variable marker in template value")]
    EmptyVariableMarker,

    #[error("template references undeclared variable: {0}")]
    UndeclaredVariable(String),

    #[error("missing required variable value: {0}")]
    MissingVariable(String),

    #[error("unknown variable provided: {0}")]
    UnknownVariable(String),

    #[error("rendered template contains conflicting locks")]
    ConflictingLocks,
}

fn parse_template_tokens(template: &str) -> Result<Vec<String>, TaskTemplateError> {
    let mut rest = template;
    let mut tokens = vec![];

    while let Some(start) = rest.find("{{") {
        let after_start = &rest[(start + 2)..];
        let Some(end_rel) = after_start.find("}}") else {
            return Err(TaskTemplateError::UnclosedVariableMarker);
        };

        let token = after_start[..end_rel].trim();

        if token.is_empty() {
            return Err(TaskTemplateError::EmptyVariableMarker);
        }

        tokens.push(token.to_string());
        rest = &after_start[(end_rel + 2)..];
    }

    Ok(tokens)
}

fn ensure_declared_tokens(
    template: &str,
    declared_variables: &HashSet<String>,
) -> Result<(), TaskTemplateError> {
    for token in parse_template_tokens(template)? {
        if !declared_variables.contains(&token) {
            return Err(TaskTemplateError::UndeclaredVariable(token));
        }
    }

    Ok(())
}

fn render_template(
    template: &str,
    declared_variables: &HashSet<String>,
    resolved_variables: &HashMap<String, String>,
) -> Result<String, TaskTemplateError> {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);

        let after_start = &rest[(start + 2)..];
        let Some(end_rel) = after_start.find("}}") else {
            return Err(TaskTemplateError::UnclosedVariableMarker);
        };

        let token = after_start[..end_rel].trim();

        if token.is_empty() {
            return Err(TaskTemplateError::EmptyVariableMarker);
        }

        if !declared_variables.contains(token) {
            return Err(TaskTemplateError::UndeclaredVariable(token.to_string()));
        }

        let Some(value) = resolved_variables.get(token) else {
            return Err(TaskTemplateError::MissingVariable(token.to_string()));
        };

        rendered.push_str(value);
        rest = &after_start[(end_rel + 2)..];
    }

    rendered.push_str(rest);

    Ok(rendered)
}

impl TaskTemplate {
    pub fn validate(&self) -> Result<(), TaskTemplateError> {
        if self.name.trim().is_empty() {
            return Err(TaskTemplateError::EmptyName);
        }

        let mut declared_variables = HashSet::new();

        for variable in &self.variables {
            let variable_name = variable.name.trim();

            if variable_name.is_empty() {
                return Err(TaskTemplateError::EmptyVariableName);
            }

            if !declared_variables.insert(variable_name.to_string()) {
                return Err(TaskTemplateError::DuplicateVariable(
                    variable_name.to_string(),
                ));
            }
        }

        ensure_declared_tokens(&self.display_name_template, &declared_variables)?;
        ensure_declared_tokens(&self.flake_ref.flake, &declared_variables)?;

        for flake_arg in &self.flake_ref.args {
            ensure_declared_tokens(flake_arg, &declared_variables)?;
        }

        for lock in &self.locks {
            ensure_declared_tokens(&lock.name, &declared_variables)?;
        }

        if let Some(group) = &self.group {
            ensure_declared_tokens(group, &declared_variables)?;
        }

        Ok(())
    }

    pub fn instantiate(
        &self,
        mut variables: HashMap<String, String>,
        needs_confirmation: bool,
    ) -> Result<Task, TaskTemplateError> {
        self.validate()?;

        let declared_variables: HashSet<String> = self
            .variables
            .iter()
            .map(|variable| variable.name.clone())
            .collect();

        for key in variables.keys() {
            if !declared_variables.contains(key) {
                return Err(TaskTemplateError::UnknownVariable(key.to_string()));
            }
        }

        let mut resolved_variables = HashMap::new();

        for variable in &self.variables {
            if let Some(value) = variables.remove(&variable.name) {
                resolved_variables.insert(variable.name.clone(), value);
                continue;
            }

            if let Some(default_value) = &variable.default_value {
                resolved_variables.insert(variable.name.clone(), default_value.clone());
                continue;
            }

            return Err(TaskTemplateError::MissingVariable(variable.name.clone()));
        }

        let display_name = render_template(
            &self.display_name_template,
            &declared_variables,
            &resolved_variables,
        )?;
        let flake = render_template(
            &self.flake_ref.flake,
            &declared_variables,
            &resolved_variables,
        )?;

        let flake_args = self
            .flake_ref
            .args
            .iter()
            .map(|arg| render_template(arg, &declared_variables, &resolved_variables))
            .collect::<Result<Vec<_>, _>>()?;

        let locks = self
            .locks
            .iter()
            .map(|lock| {
                Ok(Lock {
                    name: render_template(&lock.name, &declared_variables, &resolved_variables)?,
                    kind: lock.kind,
                    poisoned_by: None,
                })
            })
            .collect::<Result<Vec<_>, TaskTemplateError>>()?;

        let group = self
            .group
            .as_ref()
            .map(|group| render_template(group, &declared_variables, &resolved_variables))
            .transpose()?;

        let status = if needs_confirmation {
            TaskStatus::NeedsUserValidation
        } else {
            TaskStatus::New
        };

        let task = Task::builder()
            .status(status)
            .display_name(display_name)
            .flake(flake)
            .flake_args(flake_args)
            .locks(locks)
            .requires_features(self.features.clone())
            .maybe_group(group)
            .build()
            .map_err(|_| TaskTemplateError::ConflictingLocks)?;

        Ok(task)
    }
}

impl AsRef<TaskTemplate> for TaskTemplate {
    fn as_ref(&self) -> &TaskTemplate {
        self
    }
}

impl From<DbTaskTemplateLock> for TaskTemplateLock {
    fn from(lock: DbTaskTemplateLock) -> Self {
        Self {
            name: lock.name_template,
            kind: lock.lock_type,
        }
    }
}

impl From<DbTaskTemplateVariable> for TaskTemplateVariable {
    fn from(variable: DbTaskTemplateVariable) -> Self {
        Self {
            name: variable.name,
            default_value: variable.default_value,
            description: variable.description,
        }
    }
}

impl
    From<(
        DbTaskTemplate,
        Vec<DbTaskTemplateLock>,
        Vec<DbTaskTemplateVariable>,
    )> for TaskTemplate
{
    fn from(
        value: (
            DbTaskTemplate,
            Vec<DbTaskTemplateLock>,
            Vec<DbTaskTemplateVariable>,
        ),
    ) -> Self {
        let (template, locks, variables) = value;

        TaskTemplate {
            id: template.id,
            name: template.name,
            display_name_template: template.display_name_template,
            flake_ref: FlakeRef {
                flake: template.flake_ref_uri_template,
                args: template.flake_ref_args_template,
            },
            locks: locks.into_iter().map(TaskTemplateLock::from).collect(),
            features: template.features,
            group: template.group,
            variables: variables
                .into_iter()
                .map(TaskTemplateVariable::from)
                .collect(),
            created_at: template.created_at,
        }
    }
}

pub mod db_impl {
    use crate::database::entities::lock::LockKind;
    use crate::database::entities::task_template::{
        TaskTemplate, TaskTemplateLock, TaskTemplateVariable,
    };
    use crate::database::schema::{task_template_locks, task_template_variables, task_templates};
    use crate::errors::VickyError;
    use chrono::{DateTime, Utc};
    use diesel::{
        AsChangeset, Connection, ExpressionMethods, Identifiable, Insertable, OptionalExtension,
        PgConnection, QueryDsl, Queryable, RunQueryDsl, Selectable,
    };
    use itertools::Itertools;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[derive(Clone, Debug, Queryable, Selectable, Insertable, AsChangeset, Identifiable)]
    #[diesel(table_name = task_templates)]
    #[diesel(primary_key(id))]
    pub struct DbTaskTemplate {
        pub id: Uuid,
        pub name: String,
        pub display_name_template: String,
        pub flake_ref_uri_template: String,
        pub flake_ref_args_template: Vec<String>,
        pub features: Vec<String>,
        pub group: Option<String>,
        pub created_at: DateTime<Utc>,
    }

    #[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
    #[diesel(table_name = task_template_locks)]
    #[diesel(primary_key(id))]
    pub struct DbTaskTemplateLock {
        pub id: Uuid,
        pub task_template_id: Uuid,
        pub name_template: String,
        pub lock_type: LockKind,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = task_template_locks)]
    pub struct NewDbTaskTemplateLock {
        pub task_template_id: Uuid,
        pub name_template: String,
        pub lock_type: LockKind,
    }

    #[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
    #[diesel(table_name = task_template_variables)]
    #[diesel(primary_key(id))]
    pub struct DbTaskTemplateVariable {
        pub id: Uuid,
        pub task_template_id: Uuid,
        pub name: String,
        pub default_value: Option<String>,
        pub description: Option<String>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = task_template_variables)]
    pub struct NewDbTaskTemplateVariable {
        pub task_template_id: Uuid,
        pub name: String,
        pub default_value: Option<String>,
        pub description: Option<String>,
    }

    impl From<&TaskTemplate> for DbTaskTemplate {
        fn from(template: &TaskTemplate) -> Self {
            Self {
                id: template.id,
                name: template.name.clone(),
                display_name_template: template.display_name_template.clone(),
                flake_ref_uri_template: template.flake_ref.flake.clone(),
                flake_ref_args_template: template.flake_ref.args.clone(),
                features: template.features.clone(),
                group: template.group.clone(),
                created_at: template.created_at,
            }
        }
    }

    impl NewDbTaskTemplateLock {
        pub fn from_template_lock(template_id: Uuid, lock: &TaskTemplateLock) -> Self {
            Self {
                task_template_id: template_id,
                name_template: lock.name.clone(),
                lock_type: lock.kind,
            }
        }
    }

    impl NewDbTaskTemplateVariable {
        pub fn from_template_variable(template_id: Uuid, variable: &TaskTemplateVariable) -> Self {
            Self {
                task_template_id: template_id,
                name: variable.name.clone(),
                default_value: variable.default_value.clone(),
                description: variable.description.clone(),
            }
        }
    }

    pub trait TaskTemplateDatabase {
        fn get_all_task_templates(&mut self) -> Result<Vec<TaskTemplate>, VickyError>;
        fn get_task_template(
            &mut self,
            task_template_id: Uuid,
        ) -> Result<Option<TaskTemplate>, VickyError>;
        fn put_task_template(&mut self, task_template: TaskTemplate) -> Result<usize, VickyError>;
    }

    fn hydrate_templates(
        db_templates: Vec<DbTaskTemplate>,
        db_locks: Vec<DbTaskTemplateLock>,
        db_variables: Vec<DbTaskTemplateVariable>,
    ) -> Vec<TaskTemplate> {
        let mut locks_by_template: HashMap<_, Vec<DbTaskTemplateLock>> = db_locks
            .into_iter()
            .map(|lock| (lock.task_template_id, lock))
            .into_group_map();

        let mut variables_by_template: HashMap<_, Vec<DbTaskTemplateVariable>> = db_variables
            .into_iter()
            .map(|variable| (variable.task_template_id, variable))
            .into_group_map();

        db_templates
            .into_iter()
            .map(|template| {
                let locks = locks_by_template.remove(&template.id).unwrap_or_default();
                let variables = variables_by_template
                    .remove(&template.id)
                    .unwrap_or_default();
                (template, locks, variables).into()
            })
            .collect()
    }

    impl TaskTemplateDatabase for PgConnection {
        fn get_all_task_templates(&mut self) -> Result<Vec<TaskTemplate>, VickyError> {
            let db_templates = task_templates::table
                .order(task_templates::created_at.desc())
                .load::<DbTaskTemplate>(self)?;

            if db_templates.is_empty() {
                return Ok(vec![]);
            }

            let template_ids: Vec<Uuid> = db_templates.iter().map(|template| template.id).collect();

            let db_locks = task_template_locks::table
                .filter(task_template_locks::task_template_id.eq_any(&template_ids))
                .load::<DbTaskTemplateLock>(self)?;

            let db_variables = task_template_variables::table
                .filter(task_template_variables::task_template_id.eq_any(&template_ids))
                .load::<DbTaskTemplateVariable>(self)?;

            Ok(hydrate_templates(db_templates, db_locks, db_variables))
        }

        fn get_task_template(
            &mut self,
            task_template_id: Uuid,
        ) -> Result<Option<TaskTemplate>, VickyError> {
            let db_template = task_templates::table
                .filter(task_templates::id.eq(task_template_id))
                .first::<DbTaskTemplate>(self)
                .optional()?;

            let Some(db_template) = db_template else {
                return Ok(None);
            };

            let db_locks = task_template_locks::table
                .filter(task_template_locks::task_template_id.eq(task_template_id))
                .load::<DbTaskTemplateLock>(self)?;

            let db_variables = task_template_variables::table
                .filter(task_template_variables::task_template_id.eq(task_template_id))
                .load::<DbTaskTemplateVariable>(self)?;

            Ok(Some((db_template, db_locks, db_variables).into()))
        }

        fn put_task_template(&mut self, task_template: TaskTemplate) -> Result<usize, VickyError> {
            self.transaction(|conn| {
                let db_task_template = DbTaskTemplate::from(&task_template);

                let rows_updated = diesel::insert_into(task_templates::table)
                    .values(&db_task_template)
                    .execute(conn)?;

                let db_locks: Vec<NewDbTaskTemplateLock> = task_template
                    .locks
                    .iter()
                    .map(|lock| NewDbTaskTemplateLock::from_template_lock(task_template.id, lock))
                    .collect();

                if !db_locks.is_empty() {
                    diesel::insert_into(task_template_locks::table)
                        .values(&db_locks)
                        .execute(conn)?;
                }

                let db_variables: Vec<NewDbTaskTemplateVariable> = task_template
                    .variables
                    .iter()
                    .map(|variable| {
                        NewDbTaskTemplateVariable::from_template_variable(
                            task_template.id,
                            variable,
                        )
                    })
                    .collect();

                if !db_variables.is_empty() {
                    diesel::insert_into(task_template_variables::table)
                        .values(&db_variables)
                        .execute(conn)?;
                }

                Ok(rows_updated)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TaskTemplate, TaskTemplateLock, TaskTemplateVariable};
    use crate::database::entities::task::TaskStatus;
    use crate::database::entities::{LockKind, task};
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn example_template() -> TaskTemplate {
        TaskTemplate {
            id: Uuid::new_v4(),
            name: "build-something-template".to_string(),
            display_name_template: "Build {{project}} in {{env}}".to_string(),
            flake_ref: task::FlakeRef {
                flake: "nixpkgs#{{project}}".to_string(),
                args: vec!["--env={{env}}".to_string()],
            },
            locks: vec![TaskTemplateLock {
                name: "build/{{project}}".to_string(),
                kind: LockKind::Write,
            }],
            features: vec!["ijustbuildthings".to_string()],
            group: Some("{{env}}".to_string()),
            variables: vec![
                TaskTemplateVariable {
                    name: "project".to_string(),
                    default_value: None,
                    description: None,
                },
                TaskTemplateVariable {
                    name: "env".to_string(),
                    default_value: Some("production".to_string()),
                    description: None,
                },
            ],
            created_at: Utc::now(),
        }
    }

    #[test]
    fn instantiate_uses_values_and_defaults() {
        let template = example_template();

        let task = template
            .instantiate(
                HashMap::from([("project".to_string(), "chromium".to_string())]),
                false,
            )
            .expect("template should instantiate");

        assert_eq!(task.display_name, "Build chromium in production");
        assert_eq!(task.flake_ref.flake, "nixpkgs#chromium");
        assert_eq!(task.flake_ref.args, vec!["--env=production"]);
        assert_eq!(task.group, Some("production".to_string()));
        assert_eq!(task.status, TaskStatus::New);
    }

    #[test]
    fn instantiate_requires_missing_variables_without_default() {
        let template = example_template();

        let err = template
            .instantiate(HashMap::new(), false)
            .expect_err("instantiation should fail");

        assert_eq!(
            err,
            super::TaskTemplateError::MissingVariable("project".to_string())
        );
    }
}
