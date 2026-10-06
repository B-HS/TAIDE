use eframe::egui;
use taide_native_app::application::NativeApplication;
use taide_native_app::bootstrap::{LaunchConfig, restore};
use taide_runtime::TaskSupervisor;

const INITIAL_SIZE: [f32; 2] = [1400.0, 900.0];
const MINIMUM_SIZE: [f32; 2] = [720.0, 480.0];
const WINDOW_TITLE: &str = "TAIDE";

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let config = LaunchConfig::parse(arguments)?;
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
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(move |context| {
            Ok(Box::new(NativeApplication::new(
                context,
                runtime,
                state,
                tasks,
                helper_executable,
                warnings,
            )?))
        }),
    )?;
    Ok(())
}
