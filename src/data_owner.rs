use druid::Widget;

pub struct DataOwner<T, W> {
    data: T,
    widget: W,
}

impl <T, W> DataOwner<T, W> {
    pub fn new(data: T, widget: W) -> Self {
        DataOwner { data, widget }
    }
}

impl <T, U, W> Widget<U> for DataOwner<T, W> where W: Widget<T>, T: std::fmt::Debug {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &druid::Event, _data: &mut U, env: &druid::Env) {
        self.widget.event(ctx, event, &mut self.data, env);
    }

    fn lifecycle(&mut self, ctx: &mut druid::LifeCycleCtx, event: &druid::LifeCycle, _data: &U, env: &druid::Env) {
        self.widget.lifecycle(ctx, event, &self.data, env)
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old_data: &U, _data: &U, env: &druid::Env) {
        self.widget.update(ctx, &self.data, &self.data, env)
    }

    fn layout(&mut self, ctx: &mut druid::LayoutCtx, bc: &druid::BoxConstraints, _data: &U, env: &druid::Env) -> druid::Size {
        self.widget.layout(ctx, bc, &self.data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, _data: &U, env: &druid::Env) {
        self.widget.paint(ctx, &self.data, env)
    }
}
