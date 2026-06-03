use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use fs_err::File;
use karaoke::{schema::beat_to_time, score_editor::ScoreEditorData};

#[derive(Parser)]
struct Opts {
    input_json: PathBuf,
    offset: f64,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let data: ScoreEditorData = serde_json::from_reader(File::open(&opts.input_json)?)?;
    let score = &data.score;

    for track in &score.tracks {
        let Some(lyrics) = &track.lyrics else {
            continue;
        };
        let beat_to_time = |beat| beat_to_time(score.offset, &score.bpms, beat) + opts.offset;
        let [start, end] = [track.start_beat(), &track.end_beat()].map(beat_to_time);
        println!(
            "    {{ start: {start:.3}, end: {end:.3}, lyrics: `{}` }},",
            lyrics.text.trim()
        );
    }

    Ok(())
}
