use std::io::BufReader;
use std::path::PathBuf;

use clap::Parser;
use druid::AppLauncher;
use druid::WindowDesc;
use fs_err::File;
use karaoke::audio::AudioCommand;
use karaoke::audio::AudioManager;
use karaoke::config::Config;
use karaoke::fonts::FontLoader;
use karaoke::schema::Score;
use karaoke::score_editor::build_toplevel_widget;
use karaoke::score_editor::ScoreEditorData;

#[derive(Parser)]
struct Args {
    audio_path: PathBuf,
    save_path: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let args = Args::parse();

    let audio_manager = AudioManager::new()?;
    audio_manager
        .command_sender()
        .send(AudioCommand::LoadMusic(args.audio_path))
        .unwrap();
    let font_loader = FontLoader::default();

    let new_score = || ScoreEditorData::new(Score::new(config.font_path));
    let data = match args.save_path {
        Some(path) => {
            let mut score: ScoreEditorData = match File::open(&path) {
                Ok(file) => serde_json::from_reader(BufReader::new(file))?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => new_score(),
                Err(e) => return Err(e.into()),
            };
            score.save_path = path;
            score
        }
        None => new_score(),
    };
    let window = WindowDesc::new(build_toplevel_widget(audio_manager, font_loader))
        .window_size((1440.0, 810.0));
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(data)?;
    Ok(())
}
