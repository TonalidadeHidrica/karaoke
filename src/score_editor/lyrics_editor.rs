use druid::text::Selection;
use druid::widget::Button;
use druid::widget::Controller;
use druid::widget::Flex;
use druid::widget::TextBox;
use druid::Command;
use druid::Env;
use druid::Event;
use druid::EventCtx;
use druid::LifeCycle;
use druid::LifeCycleCtx;
use druid::Widget;
use druid::WidgetExt;
use druid::WidgetId;

selector! { pub UPDATE_SELECTION_SELECTOR: Option<Selection> }
selector! { pub SET_LYRICS_RANGE }
selector! { #[allow(unused)] pub NOTIFY_CURRENT_LYRICS_RANGE: Option<Selection> }

struct LyricsController {
    target_widget: WidgetId,
}

impl LyricsController {
    fn command(&self, selection: impl Into<Option<Selection>>) -> Command {
        UPDATE_SELECTION_SELECTOR
            .with(selection.into())
            .to(self.target_widget)
    }
}

impl Controller<String, TextBox<String>> for LyricsController {
    fn event(
        &mut self,
        child: &mut TextBox<String>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        child.event(ctx, event, data, env);
        let text = child.text();
        if ctx.has_focus() && text.can_read() {
            let text = text.borrow();
            ctx.submit_command(self.command(text.selection()));
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut TextBox<String>,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &String,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env);
        if let LifeCycle::FocusChanged(false) = event {
            ctx.submit_command(self.command(None));
        }
    }
}

pub fn lyrics_editor(target_widget: WidgetId) -> impl Widget<String> {
    let upper_bar = Flex::row()
        .with_child(Button::new("< Set").on_click(move |ctx, _, _| {
            ctx.submit_command(SET_LYRICS_RANGE.to(target_widget));
        }))
        .with_child(Button::new("> Get").on_click(|_, _, _| println!("Get")));
    Flex::column()
        .must_fill_main_axis(true)
        .with_child(upper_bar)
        .with_flex_child(
            TextBox::multiline()
                .controller(LyricsController { target_widget })
                .expand(),
            1.0,
        )
}
