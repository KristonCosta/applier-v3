### State
I'm going to skip implementing the monolithic `State` struct since `bevy` handles most of this for us.

### render()
Here is where we start actually using `wgpu`. In order to build out this initial rendering functionality we need to learn a bit more about `bevy` [RenderGraph](https://docs.rs/bevy_render/latest/bevy_render/render_graph/struct.RenderGraph.html)

At a high level, RenderGraph allows you to define rendering logic in composable, modular nodes. [Nodes](https://docs.rs/bevy_render/latest/bevy_render/render_graph/trait.Node.html) can be connected with [edges](https://docs.rs/bevy_render/latest/bevy_render/render_graph/enum.Edge.html) to define the topology of your rendering graph. [Slots](https://docs.rs/bevy_render/latest/bevy_render/render_graph/enum.SlotType.html) can be used to share rendering resources across nodes. 

I'd recommend reading the RenderGraph documentation to get yourself familiar with the concepts, but for now we will be focusing on `Nodes` and ignoring everything else. 

### Initial render graph
Our initial render graph is going to be exceedingly simple and just contain one node. This node will only need to do one thing, define the render pass.

To start we're going to create a plugin which will create an empty render graph.

```Rust
use bevy::{prelude::*, render::{render_graph::RenderGraphApp, RenderApp}};
pub struct ApplierPlugin;

mod graph {
	use bevy::render::render_graph::RenderSubGraph;
	
	#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
	pub struct ApplierSubgraph;
}

impl Plugin for ApplierPlugin {
	fn build(&self, app: &mut App) {
		if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app.add_render_sub_graph(graph::ApplierSubgraph);
		}
	}
}
```

Bevy completely segregates rendering systems in its own sub app. This is an important thing to keep in mind, `RenderApp` being a sub app means it does not have access to the same `world` as the main app. 

In order to define a new subgraph you need to create a subgraph label which can be done with the `RenderSubGraph` derive macro and then just add it to the `RenderApp`.  Then we can add our plugin to our `main`.

```Rust
fn main() {
	App::new()
		.add_plugins((
			ApplierPlugin,
			DefaultPlugins.set(ImagePlugin::default_nearest()),
		))
		.run();
}
```

If you run the application now you'll be greeted with the exact same black screen we saw before. We now need to define a node to add to our subgraph.

### Defining a node

To start we are going to define an empty node:
```Rust
mod node {
	use bevy::render::render_graph::Node;
	
	pub struct SurfaceNode;
	
	impl Node for SurfaceNode {
		fn run<'w>(
			&self,
			graph: &mut bevy::render::render_graph::RenderGraphContext,
			render_context: &mut bevy::render::renderer::RenderContext<'w>,
			world: &'w bevy::prelude::World,
		) -> Result<(), bevy::render::render_graph::NodeRunError> {
			todo!()
		}
	}
}
```

In order to register our node we need to define a `RenderLabel`, similar to our `RenderSubGraph`
```Rust
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum ApplierNode {
	SurfaceNode,
}
```

And then we can register our node to our subgraph
```Rust
render_app
	.add_render_sub_graph(graph::ApplierSubgraph)
	.add_render_graph_node::<SurfaceNode>(
		graph::ApplierSubgraph,
		graph::ApplierNode::SurfaceNode,
	);
```
You'll likely see an error because we didn't implement `FromWorld`, let's add that now
```Rust
impl FromWorld for SurfaceNode {
	fn from_world(world: &mut bevy::prelude::World) -> Self {
		SurfaceNode
	}
}
```

Now that we've registered our Node you may expect that if you launch the project now it would crash due to the `todo` but that's not the case, a subgraph doesn't execute unless it's explicitly called by a node in the main render graph.

One way we can do this is by creating a second node, registering it in the `RenderGraph`, and then calling `run_sub_graph` in this top level node. 

Another way is we can register our subgraph with a Bevy camera, but for now we are going to keep things simple.

### Registering our subgraph
Let's start by making another node which will execute our subgraph
```Rust
pub struct ExecuteNode;
impl Node for ExecuteNode {
	fn run<'w>(
		&self,
		graph: &mut bevy::render::render_graph::RenderGraphContext,
		render_context: &mut bevy::render::renderer::RenderContext<'w>,
		world: &'w bevy::prelude::World,
	) -> Result<(), bevy::render::render_graph::NodeRunError> {
		graph.run_sub_graph(ApplierSubgraph, vec![], None)?;
		Ok(())
	}
}
```

make a label for it
```Rust
pub enum ApplierNode {
	ExecuteNode,
	SurfaceNode,
}
```

and then add it as a node to the top level `RenderGraph`

```Rust
let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
render_graph.add_node(graph::ApplierNode::ExecuteNode, node::ExecuteNode);
```

Now our program fails when we run it
```
thread '<unnamed>' panicked at src/beginner/tutorial2-surface/src/plugin.rs:34:13:
not yet implemented
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
Encountered a panic in exclusive system `bevy_render::renderer::render_system`!
```

### Implementing the SurfaceNode
Now that our `SurfaceNode` is actually running we can implement our render pass.

A lot of this setup is already handled for us by Bevy. Window information is extracted from the main app into an `ExtractedWindows` resource available in the render app. This can be used to generate our texture view.
```
fn run<'w>(
	&self,
	graph: &mut bevy::render::render_graph::RenderGraphContext,
	render_context: &mut bevy::render::renderer::RenderContext<'w>,
	world: &'w bevy::prelude::World,
) -> Result<(), bevy::render::render_graph::NodeRunError> {
	let windows = world.resource::<ExtractedWindows>();
	for window in windows.values() {
		if let Some(view) = window.swap_chain_texture_view.as_ref() {
			// use the view
		}
	}
	Ok(())
}
```

For now we just iterate over all the `ExtractedWindows` and just assume that if we have multiple windows open it's okay if we render the same thing to all of them.

The `view` can then be used to make a `RenderPassColorAttachment`

```Rust
if let Some(view) = window.swap_chain_texture_view.as_ref() {
	let color_attachment = Some(RenderPassColorAttachment {
		view: view,
		resolve_target: None,
		ops: Operations {
			load: LoadOp::Clear(Color {
				r: 0.1,
				g: 0.2,
				b: 0.3,
				a: 1.0,
			}),
			store: StoreOp::Store,
		},
	});
}
```

The `Color` struct is actually from `wgpu` and isn't available in Bevy. Let's go ahead and add it as a dependency in our `Cargo.toml`.

```Rust
wgpu = { version = "0.19.3", default-features = false, features = [
  "wgsl",
  "dx12",
  "metal",
  "naga",
  "naga-ir",
  "fragile-send-sync-non-atomic-wasm",
] }
```

 Then make sure to import the correct `Color`!
```Rust
use wgpu::Color;
```

Okay, we've got our `TextureView` and our `ColorAttachment`, we can define our `RenderPass` now.

```Rust
let mut _render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
	label: Some("applied_pass"),
	color_attachments: &[color_attachment],
	depth_stencil_attachment: None,
	timestamp_writes: None,
	occlusion_query_set: None,
});

```


 Instead of needing to create a `CommandEncoder` Bevy provides us with a `RenderContext` which has the ability to create a `TrackedRenderPass`. It will also handling submitting the associated render commands and presenting the new output. 

If you try running the program you might see the blue screen we were hoping to see!...or you may see a black screen. Maybe if you try running your program a few times you'll see the screen swap between black and blue. What's going on?

There's a bevy debugging crate that will be helpful for us here `bevy_mod_debugdump`. Let's import it
```Rust
bevy_mod_debugdump = "0.10"
```

[bevy_mod_debugdump](https://github.com/jakobhellermann/bevy_mod_debugdump) generates `dot` formatted graphs of Bevy schedule graphs and render graphs and is massively helpful in debugging non-deterministic issues like this.

Let's update our `main` and generate the render graph.

```Rust
fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        ApplierPlugin,
    ));
    bevy_mod_debugdump::print_render_graph(&mut app);
}
```

Running it should give you
```json
digraph "RenderGraph" {
	"rankdir"="LR";
	"ranksep"="1.0";
	graph ["bgcolor"="#0d1117"];
	edge ["fontname"="Helvetica", "fontcolor"="white"];
	node ["shape"="plaintext", "fontname"="Helvetica", "fontcolor"="white"];
	subgraph "cluster_ApplierSubgraph" {
		"label"="ApplierSubgraph";
		"fontcolor"="red";
		graph ["style"="rounded,filled", "color"="#343a42", "fontcolor"="white"];
		"_ApplierSubgraph__ApplierSubgraphSurfaceNode" ["label"=<<TABLE STYLE="rounded"><TR><TD PORT="title" BORDER="0" COLSPAN="2">SurfaceNode<BR/><FONT COLOR="red" POINT-SIZE="10">SurfaceNode</FONT></TD></TR></TABLE>>, "color"="white", "fillcolor"="white"]
	}

	"_CameraDriverLabel" ["label"=<<TABLE STYLE="rounded"><TR><TD PORT="title" BORDER="0" COLSPAN="2">CameraDriverLabel<BR/><FONT COLOR="red" POINT-SIZE="10">CameraDriverNode</FONT></TD></TR></TABLE>>, "color"="white", "fillcolor"="white"]
	"_ExecuteNode" ["label"=<<TABLE STYLE="rounded"><TR><TD PORT="title" BORDER="0" COLSPAN="2">ExecuteNode<BR/><FONT COLOR="red" POINT-SIZE="10">ExecuteNode</FONT></TD></TR></TABLE>>, "color"="white", "fillcolor"="white"]
}
```

Which you can then visualize using [Graphviz](https://dreampuf.github.io/GraphvizOnline/#digraph%20%22RenderGraph%22%20%7B%0A%09%22rankdir%22%3D%22LR%22%3B%0A%09%22ranksep%22%3D%221.0%22%3B%0A%09graph%20%5B%22bgcolor%22%3D%22%230d1117%22%5D%3B%0A%09edge%20%5B%22fontname%22%3D%22Helvetica%22%2C%20%22fontcolor%22%3D%22white%22%5D%3B%0A%09node%20%5B%22shape%22%3D%22plaintext%22%2C%20%22fontname%22%3D%22Helvetica%22%2C%20%22fontcolor%22%3D%22white%22%5D%3B%0A%09subgraph%20%22cluster_ApplierSubgraph%22%20%7B%0A%09%09%22label%22%3D%22ApplierSubgraph%22%3B%0A%09%09%22fontcolor%22%3D%22red%22%3B%0A%09%09graph%20%5B%22style%22%3D%22rounded%2Cfilled%22%2C%20%22color%22%3D%22%23343a42%22%2C%20%22fontcolor%22%3D%22white%22%5D%3B%0A%09%09%22_ApplierSubgraph__ApplierSubgraphSurfaceNode%22%20%5B%22label%22%3D%3C%3CTABLE%20STYLE%3D%22rounded%22%3E%3CTR%3E%3CTD%20PORT%3D%22title%22%20BORDER%3D%220%22%20COLSPAN%3D%222%22%3ESurfaceNode%3CBR%2F%3E%3CFONT%20COLOR%3D%22red%22%20POINT-SIZE%3D%2210%22%3ESurfaceNode%3C%2FFONT%3E%3C%2FTD%3E%3C%2FTR%3E%3C%2FTABLE%3E%3E%2C%20%22color%22%3D%22white%22%2C%20%22fillcolor%22%3D%22white%22%5D%0A%09%7D%0A%0A%09%22_CameraDriverLabel%22%20%5B%22label%22%3D%3C%3CTABLE%20STYLE%3D%22rounded%22%3E%3CTR%3E%3CTD%20PORT%3D%22title%22%20BORDER%3D%220%22%20COLSPAN%3D%222%22%3ECameraDriverLabel%3CBR%2F%3E%3CFONT%20COLOR%3D%22red%22%20POINT-SIZE%3D%2210%22%3ECameraDriverNode%3C%2FFONT%3E%3C%2FTD%3E%3C%2FTR%3E%3C%2FTABLE%3E%3E%2C%20%22color%22%3D%22white%22%2C%20%22fillcolor%22%3D%22white%22%5D%0A%09%22_ExecuteNode%22%20%5B%22label%22%3D%3C%3CTABLE%20STYLE%3D%22rounded%22%3E%3CTR%3E%3CTD%20PORT%3D%22title%22%20BORDER%3D%220%22%20COLSPAN%3D%222%22%3EExecuteNode%3CBR%2F%3E%3CFONT%20COLOR%3D%22red%22%20POINT-SIZE%3D%2210%22%3EExecuteNode%3C%2FFONT%3E%3C%2FTD%3E%3C%2FTR%3E%3C%2FTABLE%3E%3E%2C%20%22color%22%3D%22white%22%2C%20%22fillcolor%22%3D%22white%22%5D%0A%7D) 
![[unexpected_node_viz.png]]
Our `ExecuteNode` and `ApplierSubgraph` looks good, but what's this `CameraDriverLabel`? 

`bevy_render` includes a `Camera` that is renderer agnostic. By default a `CameraDriverNode` is used to execute any subgraphs that a `Camera` depends upon. The specifics don't really matter too much here since we will be getting into more detail about `Camera` in lesson 6. 

The key thing is this node clears the screen with black if there are no Camera entities present [link](https://github.com/bevyengine/bevy/blob/1b3c2b0fed4821d2a8a7554330310ae7f675373d/crates/bevy_render/src/camera/camera_driver_node.rs#L58)!

The non-determinism we are seeing is caused by our `ExecuteNode` competing with the `CameraDriverNode`. Which ever `Node` is executed last wins. 

In order to fix this all we need to do is remove the `CameraDriverLabel` from the `RenderGraph` in our plugin.
```Rust
if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
	let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
	render_graph.add_node(graph::ApplierNode::ExecuteNode, node::ExecuteNode);
	render_graph
		.remove_node(CameraDriverLabel)
		.expect("failed to remove camera driver");
```

Remove the `bevy_mod_debugdump` call from your `main` function. Now when you run your program you should consistently see the blue screen we were hoping to see!

![[blue_screen_success.png]]

### Challenge
Bevy makes it easy for us to capture mouse events. The Unofficial Bevy Cheat Book has an [example](https://bevy-cheatbook.github.io/input/mouse.html#mouse-cursor-position) of how we would capture mouse events. Let's capture those events and publish them to a `Resource`:

```Rust
#[derive(Resource, Debug)]
pub struct MousePosition(f32, f32);

fn cursor_events(
    mut events: EventReader<CursorMoved>,
    mut current_position: ResMut<MousePosition>,
) {
    for event in events.read() {
        current_position.0 = event.position.x;
        current_position.1 = event.position.y;
    }
}
```

and then register the resource and our new system to the main app:
```Rust
impl Plugin for ApplierPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MousePosition(0.0, 0.0))
            .add_systems(Update, (cursor_events,));
        ...
```

Our `SurfaceNode` has access to the `world` so let's just grab the `MousePosition` from it and use it:

```Rust
let windows = world.resource::<ExtractedWindows>();
let mouse_position = world.resource::<MousePosition>();
for window in windows.values() {
	if let Some(view) = window.swap_chain_texture_view.as_ref() {
		let color_attachment = Some(RenderPassColorAttachment {
			view: view,
			resolve_target: None,
			ops: Operations {
				load: LoadOp::Clear(Color {
					r: (mouse_position.0 as f64 / window.physical_width as f64),
					g: (mouse_position.1 as f64 / window.physical_height as f64),
					b: 0.3,
					a: 1.0,
				}),
				store: StoreOp::Store,
			},
		});
	...

```

If you try to run this you'll end up with a panic
```
Requested resource tutorial2_surface::plugin::MousePosition does not exist in the `World`.
                Did you forget to add it using `app.insert_resource` / `app.init_resource`?
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

This is because we just registered our resource with the main app, not with the render app! In order to give the render app access to our resource we need to register it.

```Rust
if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
	render_app.insert_resource(MousePosition(0.0, 0.0));
```

If you run this then your screen will be a nice dark blue but it won't change. This is because we aren't actually extracting the value from the main app, we just initialized a new resource in the render app that's static. 

We need to `extract` the `Resource` from the main app if we want the background color to update. The `extract` schedule is unique, it allows read-only access to the main app while providing read and write access to the render app. `Extract` is how we get access to the main app:

```Rust
fn extract_mouse_position(
    mut mouse_position: ResMut<MousePosition>,
    main_mouse_position: Extract<Res<MousePosition>>,
) {
    mouse_position.0 = main_mouse_position.0;
    mouse_position.1 = main_mouse_position.1;
}
```

and then we just need to schedule this new system in the render app. The render app has a `ExtractSchedule` which is specifically used for extracting data from the main app.

```Rust
if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
	render_app
		.insert_resource(MousePosition(0.0, 0.0))
		.add_systems(ExtractSchedule, extract_mouse_position);
	...
```

Now when you run the program the background should respond to your cursor moving!

![[colorful_background.png]]