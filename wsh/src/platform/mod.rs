#[cfg(feature = "std")]
pub mod linux;
#[cfg(feature = "std")]
pub use linux::*;

#[cfg(not(feature = "std"))]
pub mod wunix;
#[cfg(not(feature = "std"))]
pub use wunix::*;
