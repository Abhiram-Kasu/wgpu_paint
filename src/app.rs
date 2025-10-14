use crate::state;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, MouseButton},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{self, WindowAttributes},
};

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<state::State>,
}

impl ApplicationHandler<state::State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the
            self.state = Some(pollster::block_on(state::State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: state::State) {
        // This is where proxy.send_event() ends up
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let app_state = if let Some(state) = &mut self.state {
            state
        } else {
            return;
        };

        use winit::event::WindowEvent;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => app_state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                app_state.update();
                match app_state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = app_state.window.inner_size();
                        app_state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => app_state.handle_key(event_loop, code, state.is_pressed()),
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                // Store previous position before updating
                app_state.mandelbrot_state.prev_cursor_location =
                    app_state.mandelbrot_state.cursor_location;

                // Update current position (normalized to [0, 1])
                let window_size = app_state.window.inner_size();
                app_state.mandelbrot_state.cursor_location = [
                    position.x / window_size.width as f64,
                    position.y / window_size.height as f64,
                ];

                // If we're dragging, pan the view
                if app_state.mandelbrot_state.dragging {
                    let delta_x = app_state.mandelbrot_state.cursor_location[0]
                        - app_state.mandelbrot_state.prev_cursor_location[0];
                    let delta_y = app_state.mandelbrot_state.cursor_location[1]
                        - app_state.mandelbrot_state.prev_cursor_location[1];

                    // Convert screen delta to complex plane delta
                    let window_size = app_state.window.inner_size();
                    let aspect_ratio = window_size.width as f32 / window_size.height as f32;
                    let scale = 2.0 / app_state.mandelbrot_state.zoom;

                    app_state.mandelbrot_state.center[0] -= delta_x as f32 * aspect_ratio * scale;
                    app_state.mandelbrot_state.center[1] -= delta_y as f32 * scale; // Flip Y

                    app_state.mandelbrot_state.needs_update = true;
                    app_state.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    app_state.mandelbrot_state.dragging = state.is_pressed();
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                ..
            } => {
                use winit::event::MouseScrollDelta;
                let zoom_factor = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if y > 0.0 {
                            1.2
                        } else {
                            1.0 / 1.2
                        }
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        if pos.y > 0.0 {
                            1.1
                        } else {
                            1.0 / 1.1
                        }
                    }
                };

                app_state.mandelbrot_state.zoom *= zoom_factor;
                app_state.mandelbrot_state.needs_update = true;
                app_state.window.request_redraw();
            }
            _ => {}
        }
    }
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &winit::event_loop::EventLoop<state::State>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            state: None,
        }
    }
}
