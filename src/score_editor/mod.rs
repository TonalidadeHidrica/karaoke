mod bpm_detector;
mod bpm_dialog;
mod commands;
mod data;
mod formatting;
mod layouts;
mod measure_dialog;
mod misc;
mod score_editor_widget;

use crate::audio::AudioManager;
use crate::schema::BeatLength;
use crate::schema::Score;
use druid::text::ParseFormatter;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::Scroll;
use druid::widget::Slider;
use druid::widget::TextBox;
use druid::Insets;
use druid::Widget;
use druid::WidgetExt;
use num::BigRational;

use self::formatting::beat_label_string;
use self::formatting::format_time;
use self::score_editor_widget::ScoreEditor;

pub use self::data::ScoreEditorData;

pub fn build_toplevel_widget(audio_manager: AudioManager) -> impl Widget<ScoreEditorData> {
    let status_bar = Flex::row()
        .with_child(
            Label::dynamic(|data: &ScoreEditorData, _| beat_label_string(data)).fix_width(50.0),
        )
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|len: &BeatLength, _| {
                format!("{}-th note", BigRational::from_integer(4.into()) / &len.0)
            })
            .fix_width(80.0)
            .lens(ScoreEditorData::cursor_delta),
        )
        .with_spacer(20.0)
        .with_child(
            Label::dynamic(|data: &ScoreEditorData, _| {
                let display_time = match &data.music_playback_position {
                    Some(pos) => pos.time,
                    None => data.score.beat_to_time(&data.cursor_position),
                };
                format_time(display_time)
            })
            .fix_width(80.0),
        )
        .with_spacer(20.0)
        .with_child(Label::new("Offset:"))
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .update_data_while_editing(true)
                .lens(Score::offset)
                .lens(ScoreEditorData::score),
        )
        .with_spacer(20.0)
        .with_child(Label::new("Music vol:"))
        .with_child(Slider::new().lens(ScoreEditorData::music_volume))
        .with_spacer(5.0)
        .with_child(Label::new("Metronome vol:"))
        .with_child(Slider::new().lens(ScoreEditorData::metronome_volume))
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .must_fill_main_axis(true)
        .padding(5.0);

    let score_editor = ScoreEditor {
        audio_manager,
        layout_cache: Vec::new(),
        hover_cursor: None,
    };

    Flex::column().with_child(status_bar).with_flex_child(
        Scroll::new(score_editor.padding(Insets::uniform(8.0)))
            .vertical()
            .expand_height(),
        1.0,
    )
}
