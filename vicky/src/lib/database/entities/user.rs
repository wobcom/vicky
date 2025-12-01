use crate::database::schema::users;
use diesel::{
    AsChangeset, AsExpression, FromSqlRow, Identifiable, Insertable, Queryable, Selectable,
};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, FromSqlRow, AsExpression,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[diesel(sql_type = db_impl::RoleSqlType)]
pub enum Role {
    Admin,
}

#[allow(dead_code)]
#[derive(
    Debug, Clone, Selectable, Insertable, Identifiable, Queryable, AsChangeset, Deserialize,
)]
#[diesel(table_name = users)]
#[diesel(primary_key(id))]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Me {
    pub full_name: String,
    pub role: Role,
}

pub mod db_impl {
    use crate::{database::schema::users, errors::VickyError};
    use std::io::Write;
    use uuid::Uuid;

    use crate::database::entities::user::{Role, User};
    use diesel::deserialize::FromSql;
    use diesel::pg::PgValue;
    use diesel::serialize::{IsNull, Output, ToSql};
    use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqlType};

    #[derive(SqlType)]
    #[diesel(postgres_type(name = "Role_Type"))]
    pub struct RoleSqlType;

    impl ToSql<RoleSqlType, diesel::pg::Pg> for Role {
        fn to_sql<'b>(
            &'b self,
            out: &mut Output<'b, '_, diesel::pg::Pg>,
        ) -> diesel::serialize::Result {
            match *self {
                Role::Admin => out.write_all(b"admin")?,
            }
            Ok(IsNull::No)
        }
    }

    impl FromSql<RoleSqlType, diesel::pg::Pg> for Role {
        fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
            match bytes.as_bytes() {
                b"admin" => Ok(Role::Admin),
                _ => Err("Invalid Role".into()),
            }
        }
    }

    pub trait UserDatabase {
        fn get_user(&mut self, sub: Uuid) -> Result<Option<User>, VickyError>;
        fn upsert_user(&mut self, user: User) -> Result<(), VickyError>;
    }

    impl UserDatabase for diesel::pg::PgConnection {
        fn get_user(&mut self, sub_: Uuid) -> Result<Option<User>, VickyError> {
            let db_task: Option<User> = users::table
                .filter(users::id.eq(sub_))
                .first(self)
                .optional()?;
            Ok(db_task)
        }

        fn upsert_user(&mut self, user: User) -> Result<(), VickyError> {
            let _ = diesel::insert_into(users::table)
                .values(&user)
                .on_conflict(users::id)
                .do_update()
                .set(&user)
                .execute(self);
            Ok(())
        }
    }
}
