use druid::widget::Controller;
use druid::Env;
use druid::Event;
use druid::EventCtx;
use druid::Widget;

pub struct RegisterFocus;

impl<T, W> Controller<T, W> for RegisterFocus
where
    W: Widget<T>,
{
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::WindowConnected = event {
            ctx.request_focus();
        }
        child.event(ctx, event, data, env);
    }
}
