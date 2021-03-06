use bootloader::boot_info::PixelFormat;
use conquer_once::{TryGetError, TryInitError};
use core::{fmt, num::TryFromIntError, panic::Location};
use mikanos_usb::CxxError;
use x86_64::structures::paging::{mapper::MapToError, page::AddressNotAligned, Size4KiB};

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

#[derive(Debug)]
pub(crate) enum ErrorKind {
    AddressNotAligned(AddressNotAligned),
    MapTo(MapToError<Size4KiB>),
    TryInit(TryInitError),
    TryGet(TryGetError),
    TryFromInt(TryFromIntError),
    FrameBufferNotSupported,
    PhysicalMemoryNotMapped,
    RsdpNotMapped,
    InvalidRsdp,
    InvalidXsdt,
    FadtNotFound,
    UnsupportedPixelFormat(PixelFormat),
    Deadlock,
    Full,
    NoEnoughMemory,
    XhcNotFound,
    IndexOutOfRange,
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
            ErrorKind::AddressNotAligned(err) => write!(f, "{}", err),
            ErrorKind::MapTo(err) => write!(f, "{:?}", err),
            ErrorKind::TryInit(err) => write!(f, "{}", err),
            ErrorKind::TryGet(err) => write!(f, "{}", err),
            ErrorKind::UnsupportedPixelFormat(pixel_format) => {
                write!(f, "unsupported pixel format: {:?}", pixel_format)
            }
            ErrorKind::Full => write!(f, "buffer full"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl From<AddressNotAligned> for Error {
    #[track_caller]
    fn from(err: AddressNotAligned) -> Self {
        Error::from(ErrorKind::AddressNotAligned(err))
    }
}

impl From<MapToError<Size4KiB>> for Error {
    #[track_caller]
    fn from(err: MapToError<Size4KiB>) -> Self {
        Error::from(ErrorKind::MapTo(err))
    }
}

impl From<TryInitError> for Error {
    #[track_caller]
    fn from(err: TryInitError) -> Self {
        Error::from(ErrorKind::TryInit(err))
    }
}

impl From<TryGetError> for Error {
    #[track_caller]
    fn from(err: TryGetError) -> Self {
        Error::from(ErrorKind::TryGet(err))
    }
}

impl From<TryFromIntError> for Error {
    #[track_caller]
    fn from(err: TryFromIntError) -> Self {
        Error::from(ErrorKind::TryFromInt(err))
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
