### Which crates do we need?
Well, we definitely need `bevy` but we don't want any of the existing render graphs. `bevy_core_pipeline` defines the default 2D and 3D rendering pipelines in bevy so we want to ensure we don't include those.

With that in mind, we are going to ignore default features and import only what we need.
```
[dependencies]
bevy = { version = "0.13.0", default-features = false, features = [
  "bevy_asset",
  "bevy_winit",
  "bevy_render",
  "multi-threaded",
] }
```

That's pretty much all we need to get started.

### Initial Window
Creating the initial window is painfully easy with Bevy

```
use bevy::prelude::*;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
		.run();
}
```

then you can run the project
```
cargo run
```

![[basic_window.png]]