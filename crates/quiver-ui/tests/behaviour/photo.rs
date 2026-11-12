use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{div, px, Context, TestAppContext, Window};
use quiver_ui::Photo;

use crate::open;

/// A 1x1 PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc, 0xcf, 0xc0, 0x50,
    0x0f, 0x00, 0x04, 0x85, 0x01, 0x80, 0x84, 0xa9, 0x8c, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

struct Host {
    path: PathBuf,
}

impl Render for Host {
    fn render(
        &mut self,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> impl IntoElement {
        div().relative().size(px(100.)).child(
            Photo::new(self.path.clone()).stand_in(
                div()
                    .absolute()
                    .size_full()
                    .debug_selector(|| "stand-in".into()),
            ),
        )
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quiver-photo-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("image.png")
}

#[gpui::test]
fn the_stand_in_is_drawn_until_the_image_has_loaded_and_never_after(cx: &mut TestAppContext) {
    let path = scratch("loads");
    std::fs::write(&path, PNG).unwrap();
    let (_host, cx) = open(cx, move |_, _| Host { path });
    assert!(
        cx.debug_bounds("stand-in").is_some(),
        "the first frame is drawn before the image can have loaded"
    );
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("stand-in").is_none(),
        "the loaded image stands alone: the stand-in is not drawn under it"
    );
}

#[gpui::test]
fn the_stand_in_stays_when_the_image_cannot_be_loaded(cx: &mut TestAppContext) {
    let path = scratch("missing").with_file_name("absent.png");
    let (_host, cx) = open(cx, move |_, _| Host { path });
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("stand-in").is_some(),
        "a failed image leaves the stand-in showing"
    );
}
