use rocket::FromForm;

#[derive(FromForm, Default, Clone)]
pub struct FilterParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
