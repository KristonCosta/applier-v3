mod plugin;

use bevy::prelude::*;
use plugin::ApplierPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            ApplierPlugin,
        ))
        .run();
}
