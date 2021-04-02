use druid::widget::ControllerHost;
use druid::Data;
use druid::Widget;
use druid::WidgetExt;

use super::data_owner::DataOwner;
use super::registering_focus::RegisterFocus;

pub trait WidgetExtExt<T: Data>: Widget<T> + Sized + 'static {
    fn owning_data<U: Data>(self, data: U) -> DataOwner<U, Self> {
        DataOwner::new(data, self)
    }

    fn registering_focus(self) -> ControllerHost<Self, RegisterFocus> {
        self.controller(RegisterFocus)
    }
}

impl<T: Data, W: Widget<T> + 'static> WidgetExtExt<T> for W {}
