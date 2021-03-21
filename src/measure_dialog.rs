use druid::text::format::ParseFormatter;
use druid::widget::Button;
use druid::widget::Flex;
use druid::widget::Label;
use druid::widget::TextBox;
use druid::Data;
use druid::Lens;
use druid::Widget;
use druid::WidgetExt;
use num::BigInt;

use crate::data_owner::DataOwner;

#[derive(Clone, Debug, derive_more::From, derive_more::FromStr, derive_more::Display)]
pub struct BigIntData(BigInt);

impl Data for BigIntData {
    fn same(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug, Data, Lens)]
pub struct MeasureLength {
    numerator: BigIntData,
    denominator: BigIntData,
}

impl Default for MeasureLength {
    fn default() -> Self {
        MeasureLength {
            numerator: BigIntData(4.into()),
            denominator: BigIntData(4.into()),
        }
    }
}

pub fn build_measure_dialog<T>() -> impl Widget<T>
where
    T: Data + std::fmt::Debug,
{
    let editor = Flex::row()
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .lens(MeasureLength::numerator),
        )
        .with_child(Label::new("/"))
        .with_child(
            TextBox::new()
                .with_formatter(ParseFormatter::new())
                .lens(MeasureLength::denominator),
        );

    let flex = Flex::column()
        .with_child(editor)
        .with_child(Button::new("Submit"));

    DataOwner::new(MeasureLength::default(), flex)
}
