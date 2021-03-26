use druid::AppLauncher;
use druid::WindowDesc;
use karaoke::audio::start_audio_thread;
use karaoke::error::EditorError;
use karaoke::score_editor::build_toplevel_widget;
use karaoke::score_editor::ScoreEditorData;

fn main() -> Result<(), EditorError> {
    let _audio = start_audio_thread()?;
    let data = ScoreEditorData::default();
    let window = WindowDesc::new(build_toplevel_widget).window_size((1440.0, 810.0));
    AppLauncher::with_window(window).launch(data)?;
    Ok(())
}
