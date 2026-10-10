/// An unknown error which we must refer to the logs produced by
/// libkrun to troubleshoot.
#[derive(Debug, thiserror::Error)]
#[error("unknown error; see logs for more information")]
pub struct ReferToLog;

pub(crate) fn fallible(status: i32) -> Option<i32> {
    if status < 0 { None } else { Some(status) }
}
