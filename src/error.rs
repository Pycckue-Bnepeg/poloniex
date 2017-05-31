use ws::Error;

#[derive(Debug)]
pub enum PoloniexError {
    CannotConnect,
    WSError(Error),
}