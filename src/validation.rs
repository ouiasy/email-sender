use garde::Validate;
use serde::Serialize;
use crate::errors::AppError;


#[derive(Validate, Clone, Serialize, Debug)]
pub struct ValidatedEmail(
    #[garde(email)]
    pub(crate) String
);

impl ValidatedEmail {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let email = ValidatedEmail(s.to_string());
        email.validate()?; // todo 7/17
        Ok(email)
    }
}