pub mod prelude {
    pub use nkcore_base::prelude::*;

    #[cfg(windows)]
        pub use nkcore_windows::prelude::*;
}

pub mod os {
    #[cfg(windows)]
        pub use nkcore_windows as windows;
}

pub use nkcore_base::*;

#[cfg(feature = "debug")]
    pub mod debug {
        pub use nkcore_debug::context;
        pub use nkcore_debug::api_call;
        pub use nkcore_debug::api_name_of;
    }
