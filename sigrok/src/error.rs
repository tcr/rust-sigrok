use crate::util::raw_error_code::*;
use displaydoc::Display;
use std::ffi::NulError;
use thiserror::Error;

#[derive(Error, Debug, Display)]
#[non_exhaustive]
pub enum SigrokError {
    /// Generic/unspecified error
    Err,
    /// Malloc/calloc/realloc error
    Malloc,
    /// Function argument error
    Arg,
    /// Errors hinting at internal bugs
    Bug,
    /// Incorrect samplerate
    SampleRate,
    /// Not applicable
    NA,
    /// Device is closed, but must be open
    DevClosed,
    /// A timeout occurred
    Timeout,
    /// A channel group must be specified
    ChannelGroup,
    /// Data is invalid.
    Data,
    /// Input/output error
    IO,
    /// Unknown error
    Unknown,
    /// A null byte was found when it must be non-null
    NullError(#[from] NulError),
    /// Failed to acquire the GLib main context
    GlibAcquireError,
}
impl SigrokError {
    pub(crate) fn from(code: i32) -> Result<(), SigrokError> {
        match code {
            SR_OK => Ok(()),
            SR_ERR => Err(SigrokError::Err),
            SR_ERR_MALLOC => Err(SigrokError::Malloc),
            SR_ERR_ARG => Err(SigrokError::Arg),
            SR_ERR_BUG => Err(SigrokError::Bug),
            SR_ERR_SAMPLERATE => Err(SigrokError::SampleRate),
            SR_ERR_NA => Err(SigrokError::NA),
            SR_ERR_DEV_CLOSED => Err(SigrokError::DevClosed),
            SR_ERR_TIMEOUT => Err(SigrokError::Timeout),
            SR_ERR_CHANNEL_GROUP => Err(SigrokError::ChannelGroup),
            SR_ERR_DATA => Err(SigrokError::Data),
            SR_ERR_IO => Err(SigrokError::IO),
            _ => Err(SigrokError::Unknown),
        }
    }
}
