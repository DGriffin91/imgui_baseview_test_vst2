//! Barebones baseview imgui plugin

#[macro_use]
extern crate vst;

use imgui::*;

use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{Category, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;

use imgui_baseview::{HiDpiMode, ImguiWindow, RenderSettings, Settings};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use std::sync::Arc;

const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 512;

struct TestPluginEditor {
    params: Arc<GainEffectParameters>,
    window_handle: Option<WindowHandle>,
    is_open: bool,
}

impl Editor for TestPluginEditor {
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        ::log::info!("Editor open");
        if self.is_open {
            return false;
        }

        self.is_open = true;

        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("imgui-baseview demo window"),
                size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            clear_color: (0.0, 0.0, 0.0),
            hidpi_mode: HiDpiMode::Default,
            render_settings: RenderSettings::default(),
        };

        let window_handle = ImguiWindow::open_parented(
            &VstParent(parent),
            settings,
            self.params.clone(),
            |_context: &mut Context, _state: &mut Arc<GainEffectParameters>| {},
            |run: &mut bool, ui: &Ui, state: &mut Arc<GainEffectParameters>| {
                ui.show_demo_window(run);
                let w = Window::new("Example 1: Basic sliders")
                    .size([200.0, 200.0], Condition::Appearing)
                    .position([20.0, 20.0], Condition::Appearing);
                w.build(&ui, || {
                    let mut val = state.amplitude.get();
                    if Slider::new("Gain", 0.0, 1.0)
                        .build(&ui, &mut val)
                    {
                        state.amplitude.set(val)
                    }
                });
            },
        );

        self.window_handle = Some(window_handle);

        true
    }

    fn is_open(&mut self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        if let Some(mut window_handle) = self.window_handle.take() {
            window_handle.close();
        }
    }
}
struct GainEffectParameters {
    // The plugin's state consists of a single parameter: amplitude.
    amplitude: AtomicFloat,
}
struct TestPlugin {
    params: Arc<GainEffectParameters>,
    editor: Option<TestPluginEditor>,
}

impl Default for TestPlugin {
    fn default() -> Self {
        let params = Arc::new(GainEffectParameters::default());
        Self {
            params: params.clone(),
            editor: Some(TestPluginEditor {
                params: params.clone(),
                window_handle: None,
                is_open: false,
            }),
        }
    }
}

impl Default for GainEffectParameters {
    fn default() -> GainEffectParameters {
        GainEffectParameters {
            amplitude: AtomicFloat::new(0.5),
        }
    }
}

impl Plugin for TestPlugin {
    fn get_info(&self) -> Info {
        Info {
            name: "IMGUI Gain Effect in Rust".to_string(),
            vendor: "DGriffin".to_string(),
            unique_id: 243123072,
            version: 1,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 1,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        let log_folder = ::dirs::home_dir().unwrap().join("tmp");

        let _ = ::std::fs::create_dir(log_folder.clone());

        let log_file = ::std::fs::File::create(log_folder.join("IMGUIBaseviewTest.log")).unwrap();

        let log_config = ::simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();

        let _ = ::simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file);

        ::log_panics::init();

        ::log::info!("init");
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // Read the amplitude from the parameter object
        let amplitude = self.params.amplitude.get();
        // First, we destructure our audio buffer into an arbitrary number of
        // input and output buffers.  Usually, we'll be dealing with stereo (2 of each)
        // but that might change.
        for (input_buffer, output_buffer) in buffer.zip() {
            // Next, we'll loop through each individual sample so we can apply the amplitude
            // value to it.
            for (input_sample, output_sample) in input_buffer.iter().zip(output_buffer) {
                *output_sample = *input_sample * amplitude;
            }
        }
    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

impl PluginParameters for GainEffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.amplitude.get(),
            _ => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        match index {
            0 => self.amplitude.set(val),
            _ => (),
        }
    }

    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2}", (self.amplitude.get() - 0.5) * 2f32),
            _ => "".to_string(),
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "Amplitude",
            _ => "",
        }
        .to_string()
    }
}

struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}

plugin_main!(TestPlugin);
