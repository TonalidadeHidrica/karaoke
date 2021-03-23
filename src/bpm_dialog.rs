use druid::text::format::ParseFormatter;
use druid::widget::Button;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::TextBox;
use druid::SingleUse;
use druid::Widget;
use druid::WidgetId;

use crate::data_owner::DataOwner;
use crate::schema::BeatPosition;
use crate::schema::Bpm;
use crate::score_editor::SetBpmCommand;
use crate::score_editor::EDIT_BPM_SELECTOR;

// TODO: almost the same to build_measure_dialog
pub fn build_bpm_dialog<T>(
    widget_id: WidgetId,
    position: BeatPosition,
    bpm: Bpm,
    already_exsits: bool,
) -> impl Widget<T> {
    let editor = Flex::row().with_child(Label::new("BPM:")).with_child(
        TextBox::new()
            .with_formatter(ParseFormatter::new())
            .update_data_while_editing(true),
    );

    let position_2 = position.clone();

    let mut buttons = Flex::row();
    buttons.add_child(
        Button::new("Submit").on_click(move |ctx, bpm: &mut Bpm, _| {
            let payload = SingleUse::new(SetBpmCommand {
                position: position.clone(),
                bpm: Some(*bpm),
            });
            let command = EDIT_BPM_SELECTOR.with(payload).to(widget_id);
            ctx.submit_command(command);
            ctx.window().close();
        }),
    );
    if already_exsits {
        buttons.add_child(Button::new("Remove").on_click(move |ctx, _, _| {
            let payload = SingleUse::new(SetBpmCommand {
                position: position_2.clone(),
                bpm: None,
            });
            let command = EDIT_BPM_SELECTOR.with(payload).to(widget_id);
            ctx.submit_command(command);
            ctx.window().close();
        }))
    }
    buttons.add_child(Button::new("Cancel").on_click(|ctx, _, _| ctx.window().close()));

    let flex = Flex::column().with_child(editor).with_child(buttons);

    DataOwner::new(bpm, flex)
}
