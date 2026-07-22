//! Closure-oriented adaptation of the `winit` application event model.

use std::marker::PhantomData;

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::*,
    event_loop::ActiveEventLoop,
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand as _,
    window::WindowId,
};

/// An event forwarded to a handler running through [`EventLoopExt`].
pub enum AppEvent<T> {
    /// A window-specific event and the window that produced it.
    WindowEvent(WindowId, WindowEvent),
    /// A raw input-device event and the device that produced it.
    DeviceEvent(DeviceId, DeviceEvent),
    /// A user-defined event sent through the event-loop proxy.
    UserEvent(T),
    /// The event loop is about to block while waiting for more events.
    Idle,
    /// The event loop is shutting down.
    Exit,
}

/// Runs a `winit` event loop using a stateful closure instead of an application type.
pub trait EventLoopExt<T: 'static> {
    /// Constructs a handler after the event loop resumes, then forwards each event to it.
    ///
    /// `ctor` runs once and can use the active event loop to initialize window-bound state.
    /// The returned handler remains alive until the event loop exits.
    ///
    /// Errors reported by `winit` while running the loop are returned unchanged. Panics from
    /// either closure are propagated; suspension also panics because this Windows adapter does
    /// not support suspend/resume cycles.
    fn run_app_with<C, H>(&mut self, ctor: C) -> Result<(), EventLoopError>
    where
        C: FnOnce(&ActiveEventLoop) -> H,
        H: FnMut(&ActiveEventLoop, AppEvent<T>);
}

impl<T: 'static> EventLoopExt<T> for EventLoop<T> {
    fn run_app_with<C, H>(&mut self, ctor: C) -> Result<(), EventLoopError>
    where
        C: FnOnce(&ActiveEventLoop) -> H,
        H: FnMut(&ActiveEventLoop, AppEvent<T>) {
        self.run_app_on_demand(&mut App {
            constructor: Some(ctor),
            handler: None,
            phantom: PhantomData,
        })
    }
}

struct App<T: 'static, C, H>
where
    C: FnOnce(&ActiveEventLoop) -> H,
    H: FnMut(&ActiveEventLoop, AppEvent<T>) {
    constructor: Option<C>,
    handler: Option<H>,
    phantom: PhantomData<T>,
}

impl<T: 'static, C, H> ApplicationHandler<T> for App<T, C, H>
where
    C: FnOnce(&ActiveEventLoop) -> H,
    H: FnMut(&ActiveEventLoop, AppEvent<T>) {
    fn suspended(&mut self, _: &ActiveEventLoop) {
        // SAFETY(panic): suspension and resumption are not supported on windows.
        panic!("not supported");
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // SAFETY(unwrap): suspension and resumption are not supported on windows,
        // so resumed() is guaranteed to be called only once.
        self.handler = Some((self.constructor.take().unwrap())(event_loop));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent) {
        (self.handler.as_mut().unwrap())(
            event_loop,
            AppEvent::WindowEvent(window_id, event));
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent) {
        (self.handler.as_mut().unwrap())(
            event_loop,
            AppEvent::DeviceEvent(device_id, event));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: T) {
        (self.handler.as_mut().unwrap())(
            event_loop,
            AppEvent::UserEvent(event));
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        (self.handler.as_mut().unwrap())(event_loop, AppEvent::Idle);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        (self.handler.as_mut().unwrap())(event_loop, AppEvent::Exit);
    }
}
