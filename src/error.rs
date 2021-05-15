use crate::prelude::*;
use core::fmt;
use mikanos_usb::CxxError;
use x86_64::structures::paging::{
    mapper::MapToError, page::AddressNotAligned, PhysFrame, Size4KiB,
};

pub(crate) type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct Error {
    kind: ErrorKind,
    file: &'static str,
    line: u32,
    column: u32,
}

impl Error {
    #[doc(hidden)]
    pub(crate) fn _new(kind: ErrorKind, file: &'static str, line: u32, column: u32) -> Self {
        Self {
            kind,
            file,
            line,
            column,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: {}, {}:{}:{}",
            self.kind, self.kind, self.file, self.line, self.column
        )?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    Uninit(&'static str),
    WouldBlock(&'static str),
    Full,
    NoEnoughMemory,
    XhcNotFound,
    IndexOutOfRange,
    AddressNotAligned,
    FrameAllocationFailed,
    ParentEntryHugePage,
    PageAlreadyMapped(PhysFrame<Size4KiB>),
    InvalidSlotID,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
    EndpointNotInCharge,
    Unknown,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Uninit(target) => write!(f, "{} is uninitialized", target),
            ErrorKind::WouldBlock(target) => {
                write!(f, "{} is currently being initialized", target)
            }
            ErrorKind::Full => write!(f, "buffer full"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl From<AddressNotAligned> for Error {
    #[track_caller]
    fn from(_: AddressNotAligned) -> Self {
        make_error!(ErrorKind::AddressNotAligned)
    }
}

impl From<MapToError<Size4KiB>> for Error {
    #[track_caller]
    fn from(err: MapToError<Size4KiB>) -> Self {
        match err {
            MapToError::FrameAllocationFailed => {
                make_error!(ErrorKind::FrameAllocationFailed)
            }
            MapToError::ParentEntryHugePage => crate::make_error!(ErrorKind::ParentEntryHugePage),
            MapToError::PageAlreadyMapped(frame) => {
                make_error!(ErrorKind::PageAlreadyMapped(frame))
            }
        }
    }
}

impl From<CxxError> for Error {
    #[track_caller]
    fn from(err: CxxError) -> Self {
        use ErrorKind::*;
        let kind = match err.0 {
            1 => NoEnoughMemory,
            2 => InvalidSlotID,
            3 => InvalidEndpointNumber,
            4 => TransferRingNotSet,
            5 => AlreadyAllocated,
            6 => NotImplemented,
            7 => InvalidDescriptor,
            8 => BufferTooSmall,
            9 => UnknownDevice,
            10 => NoCorrespondingSetupStage,
            11 => TransferFailed,
            12 => InvalidPhase,
            13 => UnknownXHCISpeedID,
            14 => NoWaiter,
            15 => EndpointNotInCharge,
            _ => Unknown,
        };
        make_error!(kind)
    }
}

#[macro_export]
macro_rules! make_error {
    ($kind:expr $(,)?) => {
        $crate::error::Error::_new($kind, file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! bail {
    ($kind:expr) => {
        return Err($crate::make_error!($kind))
    };
}
