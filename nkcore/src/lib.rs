pub mod prelude {
    pub use nkcore_base::prelude::*;

    #[cfg(windows)]
        pub use nkcore_windows::prelude::*;
}

pub use nkcore_base::*;

#[cfg(feature = "debug")]
    pub use nkcore_debug::*;
#[cfg(windows)]
    pub use nkcore_windows::*;
