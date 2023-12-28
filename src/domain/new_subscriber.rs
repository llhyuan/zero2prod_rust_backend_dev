use crate::domain::subscriber_email::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

#[derive(serde::Deserialize)]
pub struct FormDataSubscriber {
    pub email: String,
    pub name: String,
}

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl TryFrom<FormDataSubscriber> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormDataSubscriber) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { name, email })
    }
}
