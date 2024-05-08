pub mod db_impl {
    use crate::{database::schema::users, errors::VickyError};
    use diesel::{Identifiable, Insertable, Queryable, Selectable, OptionalExtension, AsChangeset};
    use uuid::Uuid;

    use diesel::{
        ExpressionMethods, QueryDsl, RunQueryDsl,
    };

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
            use self::users::dsl::*;
            let db_task: Option<DbUser> = users.filter(self::users::sub.eq(sub_)).first(self).optional()?;
            Ok(db_task)
        }

        fn upsert_user(&mut self, user: DbUser) -> Result<(), VickyError> {
            use self::users::dsl::*;
            let _ = diesel::insert_into(users)
                .values(&user)
                .on_conflict(sub)
                .do_update()
                .set(&user)
                .execute(self);
            Ok(())
        }
    }
}
