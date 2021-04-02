use druid::text::format::ParseFormatter;
use druid::widget::Button;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::TextBox;
use druid::SingleUse;
use druid::Widget;
use druid::WidgetExt;
use druid::WidgetId;

use crate::druid_supplemental::widget_ext_ext::WidgetExtExt;
use crate::schema::BeatPosition;
use crate::schema::MeasureLength;
use crate::score_editor::SetMeasureLengthCommand;
use crate::score_editor::EDIT_MEAUSRE_LENGTH_SELECTOR;

pub fn build_measure_dialog<T>(
    widget_id: WidgetId,
    position: BeatPosition,
    measure_length: MeasureLength,
    already_exsits: bool,
) -> impl Widget<T> {
    let editor = Flex::row()
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .update_data_while_editing(true)
                .lens(MeasureLength::numerator),
        )
        .with_child(Label::new("/"))
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .update_data_while_editing(true)
                .lens(MeasureLength::denominator),
        );

    let position_2 = position.clone();

    let mut buttons = Flex::row();
    buttons.add_child(
        Button::new("Submit").on_click(move |ctx, data: &mut MeasureLength, _| {
            let payload = SingleUse::new(SetMeasureLengthCommand {
                position: position.clone(),
                measure_length: Some(data.clone()),
            });
            let command = EDIT_MEAUSRE_LENGTH_SELECTOR.with(payload).to(widget_id);
            ctx.submit_command(command);
            ctx.window().close();
        }),
    );
    if already_exsits {
        buttons.add_child(Button::new("Remove").on_click(move |ctx, _, _| {
            let payload = SingleUse::new(SetMeasureLengthCommand {
                position: position_2.clone(),
                measure_length: None,
            });
            let command = EDIT_MEAUSRE_LENGTH_SELECTOR.with(payload).to(widget_id);
            ctx.submit_command(command);
            ctx.window().close();
        }))
    }
    buttons.add_child(Button::new("Cancel").on_click(|ctx, _, _| ctx.window().close()));

    Flex::column()
        .with_child(editor)
        .with_child(buttons)
        .owning_data(measure_length)
}
