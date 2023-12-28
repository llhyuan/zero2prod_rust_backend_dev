mod new_subscriber;
mod subscriber_email;
mod subscriber_name;

// expose chosen features on a sub-crate level
pub use new_subscriber::FormDataSubscriber;
pub use new_subscriber::NewSubscriber;
pub use subscriber_email::SubscriberEmail;
pub use subscriber_name::SubscriberName;
