mod plugin;

use bevy::prelude::*;
use plugin::ApplierPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        ApplierPlugin,
    ));
    app.run();
}
