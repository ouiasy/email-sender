use tracing::Subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, fmt};

pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let fmt_layer = fmt::layer().with_target(false).json();

    Registry::default().with(filter_layer).with(fmt_layer)
}
