macro_rules! selector {
    ($vis: vis $id: ident: $t: ty) => {
        #[allow(unused)]
        $vis const $id: druid::Selector<$t> = druid::Selector::new(concat!(module_path!(), "::", stringify!($id)));
    };
    ($vis: vis $id: ident) => {
        selector!{ $vis $id: () }
    };
}
