use rocket::FromForm;

#[derive(FromForm, Default, Clone)]
pub struct FilterParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub group: Option<String>,
}

impl From<Option<FilterParams>> for FilterParams {
    fn from(value: Option<FilterParams>) -> Self {
        value.unwrap_or_default()
    }
}