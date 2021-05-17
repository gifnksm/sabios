use bootloader::boot_info::PixelFormat;
use conquer_once::{TryGetError, TryInitError};
use core::{fmt, panic::Location};
use mikanos_usb::CxxError;
use x86_64::structures::paging::{
    mapper::MapToError, page::AddressNotAligned, PhysFrame, Size4KiB,
};

pub(crate) type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct Error {
    kind: ErrorKind,
    location: &'static Location<'static>,
}

impl Error {
    #[track_caller]
    pub(crate) fn new(kind: ErrorKind) -> Self {
        let location = Location::caller();
        Self { kind, location }
    }
}

impl From<ErrorKind> for Error {
    #[track_caller]
    fn from(err: ErrorKind) -> Self {
        Error::new(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: {}, {}:{}:{}",
            self.kind,
            self.kind,
            self.location.file(),
            self.location.line(),
            self.location.column()
        )?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    UnsupportedPixelFormat(PixelFormat),
    ParameterTooLarge(&'static str, usize),
    AlreadyInit(&'static str),
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
    NoPciMsi,
    Unknown,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::UnsupportedPixelFormat(pixel_format) => {
                write!(f, "unsupported pixel format: {:?}", pixel_format)
            }
            ErrorKind::ParameterTooLarge(name, value) => write!(f, "too large {}: {}", name, value),
            ErrorKind::AlreadyInit(target) => write!(f, "{} has already been initialized", target),
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
        Error::from(ErrorKind::AddressNotAligned)
    }
}

impl From<MapToError<Size4KiB>> for Error {
    #[track_caller]
    fn from(err: MapToError<Size4KiB>) -> Self {
        let kind = match err {
            MapToError::FrameAllocationFailed => ErrorKind::FrameAllocationFailed,
            MapToError::ParentEntryHugePage => ErrorKind::ParentEntryHugePage,
            MapToError::PageAlreadyMapped(frame) => ErrorKind::PageAlreadyMapped(frame),
        };
        Error::from(kind)
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
        Error::from(kind)
    }
}

#[macro_export]
macro_rules! bail {
    ($err:expr) => {
        return Err($crate::error::Error::from($err))
    };
}
pub(crate) trait ConvertErr {
    type Output;
    fn convert_err(self, msg: &'static str) -> Self::Output;
}

impl<T> ConvertErr for core::result::Result<T, TryGetError> {
    type Output = core::result::Result<T, Error>;

    #[track_caller]
    fn convert_err(self, msg: &'static str) -> Self::Output {
        self.map_err(|err| {
            let kind = match err {
                TryGetError::Uninit => ErrorKind::Uninit(msg),
                TryGetError::WouldBlock => ErrorKind::WouldBlock(msg),
            };
            Error::from(kind)
        })
    }
}

impl<T> ConvertErr for core::result::Result<T, TryInitError> {
    type Output = core::result::Result<T, Error>;

    #[track_caller]
    fn convert_err(self, msg: &'static str) -> Self::Output {
        self.map_err(|err| {
            let kind = match err {
                TryInitError::AlreadyInit => ErrorKind::AlreadyInit(msg),
                TryInitError::WouldBlock => ErrorKind::WouldBlock(msg),
            };
            Error::from(kind)
        })
    }
}
