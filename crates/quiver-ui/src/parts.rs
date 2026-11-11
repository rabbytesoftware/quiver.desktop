//! `part!`: declares a styled container component (children + style override) from a palette
//! closure. Frame, Dialog and Sheet are made of such parts.

macro_rules! part {
    ($(#[$m:meta])* $name:ident, |$p:ident, $d:ident| $body:expr) => {
        $crate::parts::part!($(#[$m])* $name, |$p, $d, _q| $body);
    };
    ($(#[$m:meta])* $name:ident, |$p:ident, $d:ident, $q:ident| $body:expr) => {
        $(#[$m])*
        #[derive(gpui::IntoElement, Default)]
        pub struct $name {
            children: Vec<gpui::AnyElement>,
            style: gpui::StyleRefinement,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl gpui::Styled for $name {
            fn style(&mut self) -> &mut gpui::StyleRefinement {
                &mut self.style
            }
        }

        impl gpui::ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl gpui::RenderOnce for $name {
            fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
                let $q: $crate::QuiverTheme = *$crate::theme::ActiveQuiver::quiver(cx);
                let $p: $crate::Palette = $q.palette;
                let $d = gpui::div();
                let mut el = gpui::ParentElement::children($body, self.children);
                gpui::Styled::style(&mut el).refine(&self.style);
                el
            }
        }
    };
}

pub(crate) use part;
