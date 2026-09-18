#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[allow(dead_code)]
mod allocator;
mod buffer;
mod export;
mod parameter;
mod plugin;
#[doc(hidden)]
pub mod runtime;
#[allow(dead_code)]
mod runtime_state;
mod touch;

pub use allocator::AllocationStats;
pub use buffer::{BufferError, StereoBuffer, StereoFrame, StereoFramesMut, StereoInput};
pub use nts3_macros::{Nts3Parameters, plugin};
pub use parameter::{
    Nts3Parameters, Parameter, ParameterError, Smooth, SmoothStatus, SmoothedParameter,
};
pub use plugin::{InitContext, InitError, Nts3Plugin};
pub use touch::{TouchEvent, TouchPhase};

/// Implementation details used by generated code and internal fixtures.
#[doc(hidden)]
pub mod __private {
    pub use crate::export::{RESOURCE_MAGIC, RESOURCE_SCHEMA_VERSION, ResourceRecord};
    pub use nts3_sys::{
        GenericfxCurve, GenericfxParamMapping, GenericfxUnitHeader, UNIT_API_VERSION,
        UNIT_TARGET_NTS3_KAOSS_GENERICFX, UNUSED_MAPPING, UNUSED_PARAM, UnitHeader, UnitParam,
        UnitParamFormat, UnitRuntimeDescriptor,
    };

    pub trait Sealed {}
}

/// Common plugin-author imports.
pub mod prelude {
    pub use crate::{
        InitContext, InitError, Nts3Parameters, Nts3Plugin, Parameter, ParameterError, Smooth,
        SmoothStatus, SmoothedParameter, StereoBuffer, StereoInput, TouchEvent, TouchPhase,
    };
}

#[doc(hidden)]
pub use allocator::FrameworkAllocator;

/// Host-only initialization allocation probe support.
#[cfg(not(target_os = "none"))]
pub mod host {
    pub use crate::allocator::host::HostArena;
    pub use crate::runtime::host::{HostProbeReport, probe};
}

/// Installs the process-global target allocator and panic policy in the final
/// plugin crate. The plugin attribute invokes this exactly once; it is not part
/// of the ordinary author API.
#[doc(hidden)]
#[macro_export]
macro_rules! __install_runtime_glue {
    () => {
        #[cfg(target_os = "none")]
        #[global_allocator]
        static __NTS3_GLOBAL_ALLOCATOR: $crate::FrameworkAllocator = $crate::FrameworkAllocator;

        #[cfg(target_os = "none")]
        #[panic_handler]
        fn __nts3_panic(_information: &core::panic::PanicInfo<'_>) -> ! {
            $crate::__target_panic_fault()
        }
    };
}

#[cfg(target_os = "none")]
#[doc(hidden)]
#[cold]
#[inline(never)]
pub fn __target_panic_fault() -> ! {
    // Target policy deliberately performs no formatting, allocation or unwind.
    loop {
        core::hint::spin_loop();
    }
}
