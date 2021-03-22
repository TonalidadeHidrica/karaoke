use druid::text::format::ParseFormatter;
use druid::widget::Button;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::TextBox;
use druid::Data;
use druid::SingleUse;
use druid::Widget;
use druid::WidgetExt;
use druid::WidgetId;

use crate::data_owner::DataOwner;
use crate::schema::BeatPosition;
use crate::schema::MeasureLength;
use crate::score_editor::SetMeasureLengthCommand;
use crate::score_editor::EDIT_MEAUSRE_LENGTH_SELECTOR;

pub fn build_measure_dialog<T>(
    widget_id: WidgetId,
    position: BeatPosition,
    measure_length: MeasureLength,
) -> impl Widget<T>
where
    T: Data,
{
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

    let buttons = Flex::row()
        .with_child(
            Button::new("Submit").on_click(move |ctx, data: &mut MeasureLength, _| {
                let payload = SingleUse::new(SetMeasureLengthCommand {
                    position: position.clone(),
                    measure_length: data.clone(),
                });
                let command = EDIT_MEAUSRE_LENGTH_SELECTOR.with(payload).to(widget_id);
                ctx.submit_command(command);
                ctx.window().close();
            }),
        )
        .with_child(Button::new("Cancel").on_click(|ctx, _, _| ctx.window().close()));

    let flex = Flex::column().with_child(editor).with_child(buttons);

    DataOwner::new(measure_length, flex)
}
