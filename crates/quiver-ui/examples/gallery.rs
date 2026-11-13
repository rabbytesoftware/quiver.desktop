use std::borrow::Cow;

fn main() {
    let fonts = [
        &include_bytes!("../../quiver-app/assets/fonts/Inter-Regular.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/Inter-Medium.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/Inter-SemiBold.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/Inter-Bold.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/Inter-ExtraBold.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/JetBrainsMono-Regular.ttf")[..],
        &include_bytes!("../../quiver-app/assets/fonts/JetBrainsMono-Medium.ttf")[..],
    ];
    quiver_ui::run_gallery(fonts.iter().map(|f| Cow::Borrowed(*f)).collect());
}
