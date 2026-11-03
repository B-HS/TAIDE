use std::cell::RefCell;
use std::rc::Rc;

use eframe::egui;
use taide_native_app::application::NativeApplication;
use taide_native_app::bootstrap::{Launch, restore};
#[cfg(unix)]
use taide_native_app::single_instance;
use taide_native_app::window_state;
use taide_runtime::TaskSupervisor;

const INITIAL_SIZE: [f32; 2] = [1400.0, 900.0];
const MINIMUM_SIZE: [f32; 2] = [720.0, 480.0];
const WINDOW_TITLE: &str = "TAIDE";
const WINDOW_NOT_CREATED: &str = "native main window was not created";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    taide_native_app::stderr_log::install();
    let mut arguments = std::env::args().skip(1).peekable();
    if arguments
        .peek()
        .is_some_and(|argument| argument == taide_native_app::preview_web_helper::FLAG)
    {
        arguments.next();
        if arguments.next().is_some() {
            return Err("HTML helper does not accept additional arguments".into());
        }
        taide_native_app::preview_web_helper::serve(
            std::io::stdin().lock(),
            std::io::stdout().lock(),
        )?;
        return Ok(());
    }
    let Launch {
        config,
        relay_arguments,
        external_open,
    } = Launch::parse(arguments)?;
    #[cfg(unix)]
    let (relay, _relay_socket) = match single_instance::acquire(
        &single_instance::socket_path(&config.data_dir),
        &relay_arguments,
    ) {
        single_instance::Claim::Relayed => return Ok(()),
        single_instance::Claim::Primary(listener, socket) => (Some(listener), Some(socket)),
        single_instance::Claim::Unavailable => (None, None),
    };
    let monitors = window_state::Monitors::default();
    let window_builder = window_state::main_window_hook(config.data_dir.clone(), monitors.clone());
    let helper_executable = std::env::current_exe()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let (state, warnings) = runtime.block_on(restore(config, &tasks))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(INITIAL_SIZE)
            .with_min_inner_size(MINIMUM_SIZE)
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false),
        window_builder: Some(window_builder),
        ..Default::default()
    };
    let startup = Rc::new(RefCell::new(None::<Result<(), String>>));
    let creation = startup.clone();
    let event_loop =
        winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build()?;
    let application = eframe::create_native(
        WINDOW_TITLE,
        options,
        Box::new(move |context| {
            let application =
                NativeApplication::new(context, runtime, state, tasks, helper_executable, warnings)
                    .inspect_err(|error| *creation.borrow_mut() = Some(Err(error.to_string())))?;
            *creation.borrow_mut() = Some(Ok(()));
            #[cfg(target_os = "macos")]
            let application = application.with_app_menu(&context.egui_ctx);
            #[cfg(unix)]
            let application = application.with_relay(relay);
            Ok(Box::new(application.with_external_open(external_open)))
        }),
        &event_loop,
    );
    let mut probe = window_state::MonitorProbe::new(application, monitors);
    event_loop.run_app(&mut probe)?;
    let outcome = startup.borrow_mut().take();
    match outcome {
        Some(result) => result?,
        None => return Err(WINDOW_NOT_CREATED.into()),
    }
    Ok(probe.take_result()?)
}
