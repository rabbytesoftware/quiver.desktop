//! A demo-mode app model for the grid screens' view tests: seeded stores, no daemon.

use gpui::{Entity, TestAppContext};

use crate::model::AppModel;

pub fn demo_app(cx: &mut TestAppContext) -> Entity<AppModel> {
    let runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime"),
    ));
    let services = crate::demo::services(runtime).expect("services");
    cx.update(|cx| {
        let app = AppModel::install(cx, services, true);
        crate::demo::seed(app.read(cx));
        app
    })
}
