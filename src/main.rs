use druid::AppLauncher;
use druid::WindowDesc;
use karaoke::audio::AudioCommand;
use karaoke::audio::AudioManager;
use karaoke::error::EditorError;
use karaoke::score_editor::build_toplevel_widget;
use karaoke::score_editor::ScoreEditorData;

fn main() -> Result<(), EditorError> {
    let audio_manager = AudioManager::new()?;
    if let Some(path) = std::env::args().nth(1) {
        audio_manager
            .command_sender()
            .send(AudioCommand::LoadMusic(path.into()))
            .unwrap();
    };
    let data = ScoreEditorData::default();
    let window =
        WindowDesc::new(|| build_toplevel_widget(audio_manager)).window_size((1440.0, 810.0));
    AppLauncher::with_window(window).launch(data)?;
    Ok(())
}
