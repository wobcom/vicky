use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Role {
    Admin,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Me {
    pub full_name: String,
    pub role: Role,
}

pub mod db_impl {
    use crate::{database::schema::users, errors::VickyError};
    use diesel::{AsChangeset, Identifiable, Insertable, OptionalExtension, Queryable, Selectable};
    use uuid::Uuid;

    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

    #[derive(Selectable, Insertable, Identifiable, Queryable, AsChangeset, Debug, Clone)]
    #[diesel(table_name = users)]
    #[diesel(primary_key(sub))]
    pub struct DbUser {
        pub sub: Uuid,
        pub name: String,
        pub role: String,
    }

    pub trait UserDatabase {
        fn get_user(&mut self, sub: Uuid) -> Result<Option<DbUser>, VickyError>;
        fn upsert_user(&mut self, user: DbUser) -> Result<(), VickyError>;
    }

    impl UserDatabase for diesel::pg::PgConnection {
        fn get_user(&mut self, sub_: Uuid) -> Result<Option<DbUser>, VickyError> {
            let db_task: Option<DbUser> = users::table
                .filter(users::sub.eq(sub_))
                .first(self)
                .optional()?;
            Ok(db_task)
        }

        fn upsert_user(&mut self, user: DbUser) -> Result<(), VickyError> {
            let _ = diesel::insert_into(users::table)
                .values(&user)
                .on_conflict(users::sub)
                .do_update()
                .set(&user)
                .execute(self);
            Ok(())
        }
    }
}
