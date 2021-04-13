macro_rules! selector {
    ($vis: vis $id: ident: $t: ty) => {
        $vis const $id: druid::Selector<$t> = druid::Selector::new(concat!(module_path!(), "::", stringify!($id)));
    };
}
