use std::{
    error::Error as StdError,
    fmt::{self, Write},
};

pub use self::{
    health_check::health_check, newsletters::publish_newsletter, subscriptions::subscribe,
    subscriptions_confirm::confirm,
};

mod health_check;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

fn error_chain_msg(err: &impl StdError) -> Result<String, fmt::Error> {
    let mut msg = String::new();
    writeln!(msg, "{}\n", err)?;
    let mut current = err.source();
    while let Some(source) = current {
        writeln!(msg, "caused by:\n\t{}", source)?;
        current = source.source();
    }
    Ok(msg)
}
