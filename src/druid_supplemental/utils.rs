macro_rules! selector {
    ($(#[$m:meta])* $vis: vis $id: ident: $t: ty) => {
        $(#[$m])*
        $vis const $id: druid::Selector<$t> = druid::Selector::new(concat!(module_path!(), "::", stringify!($id)));
    };
    ($vis: vis $id: ident) => {
        selector!{ $vis $id: () }
    };
}
