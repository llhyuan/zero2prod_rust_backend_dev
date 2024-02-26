use askama::Template;

#[derive(Template)]
#[template(path = "hello.html")]

pub struct HelloTemplate<'a> {
    pub name: &'a str,
}

#[derive(Template)]
#[template(path = "health_check.html")]
pub struct HealthCheckTemplate<'a> {
    pub text: &'a str,
}
