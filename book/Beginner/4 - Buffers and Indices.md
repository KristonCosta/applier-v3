### The vertex buffer
I generally followed this verbatim. The only difference is we need to store the buffer as a resource to make it easily accessible to our pipeline.

Bevy has `BufferVec` that makes defining and initializing this buffer quite easy:

```Rust
#[derive(Resource)]
pub struct VertexBuffer(BufferVec<mesh::Vertex>);

impl FromWorld for VertexBuffer {
    fn from_world(world: &mut World) -> Self {
        let mut buff = BufferVec::new(BufferUsages::VERTEX);
        buff.extend(mesh::VERTICES.to_vec());
        Self(buff)
    }
}
```

and initialize it in our `render_app`

```Rust
...
render_app
	.insert_resource(MousePosition(0.0, 0.0))
	.init_resource::<VertexBuffer>()
...
```

We still need to actually write our `BufferVec` out to the GPU. In order to do so we need to set up a system which has access to a `RenderDevice` and `RenderQueue`. The `PrepareResources` step in the Bevy `RenderSet` is the perfect place to initialize our buffer.

```Rust
fn prepare_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut vertex_buffer: ResMut<VertexBuffer>,
) {
    vertex_buffer.0.write_buffer(&render_device, &render_queue);
}
```

and schedule it in our `render_app`

```Rust
render_app
	.insert_resource(MousePosition(0.0, 0.0))
	.init_resource::<VertexBuffer>()
	.add_systems(ExtractSchedule, extract_mouse_position)
	.add_systems(Render, prepare_buffers.in_set(RenderSet::PrepareResources));
```

### So, what do I do with it?
Define the `VertexBufferLayout` as described in the tutorial and update your pipeline definition.

Conveniently the `BufferVec` has a `len()` method we can use to get the length of the buffer without needing to track it ourselves. Then just fetch it in your `SurfaceNode` and use it!

```Rust
...
let applier_pipeline = world.resource::<ApplierPipeline>();
let vertex_buffer = world.resource::<VertexBuffer>();
...
if let Some(pipeline) = pipeline_cache.get_render_pipeline(applier_pipeline.id)
{
	render_pass.set_render_pipeline(pipeline);
	render_pass.set_vertex_buffer(
		0,
		vertex_buffer
			.0
			.buffer()
			.expect("buffer was not set")
			.slice(..),
	);
	render_pass.draw(0..vertex_buffer.0.len() as u32, 0..1);
}
```

Update your `WGSL` following the tutorial and you should see a colorful triangle when you run the code.
![[colorful_triangle.png]]

### The index buffer
This is pretty much the same as above. Just make a resource to store the index buffer, make sure to prepare it, then use it in the render command.

```Rust

#[derive(Resource)]
pub struct IndexBuffer(BufferVec<u16>);

impl FromWorld for IndexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = BufferVec::new(BufferUsages::INDEX);
        buff.extend(mesh::INDICES.to_vec());
        Self(buff)
    }
}


fn prepare_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut vertex_buffer: ResMut<VertexBuffer>,
    mut index_buffer: ResMut<IndexBuffer>,
) {
    vertex_buffer.0.write_buffer(&render_device, &render_queue);
    index_buffer.0.write_buffer(&render_device, &render_queue);
}

// Plugin::build()
render_app
    .insert_resource(MousePosition(0.0, 0.0))
                .init_resource::<VertexBuffer>()
                .init_resource::<IndexBuffer>()


// SurfaceNode::run
let index_buffer = world.resource::<IndexBuffer>();
...
render_pass.set_render_pipeline(pipeline);
render_pass.set_vertex_buffer(
	0,
	vertex_buffer
		.0
		.buffer()
		.expect("buffer was not set")
		.slice(..),
);
render_pass.set_index_buffer(
	index_buffer
		.0
		.buffer()
		.expect("buffer was not set")
		.slice(..),
	0,
	wgpu::IndexFormat::Uint16,
);
render_pass.draw_indexed(0..index_buffer.0.len() as u32, 0, 0..1)
```

Wait a second, if you run this you end up with 
```Rust
wgpu error: Validation Error

Caused by:
    In Queue::write_buffer
    Copy size 18 does not respect `COPY_BUFFER_ALIGNMENT`
```

The tutorial says alignment is automatically handled by `wgpu` why are we getting alignment errors?

Internally Bevy just uses `create_buffer` and then writes the data to the new buffer. In the tutorial `create_buffer_init` is used which automatically handles buffer alignment for us. Bevy also explicitly mentions that this [`BufferVec`](https://docs.rs/bevy/latest/bevy/render/render_resource/struct.BufferVec.html) should only be used on "Properly formatted" data. 

Since our index size is `u16` an odd number of indices will result in an unaligned buffer. We could create our own `BufferVec` implementation or similar, but instead I'm just going to adjust the index to be `u32` for automatic alignment.

```Rust 
pub const INDICES: &[u32] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];


#[derive(Resource)]
pub struct IndexBuffer(BufferVec<u32>);

impl FromWorld for IndexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = BufferVec::new(BufferUsages::INDEX);
        buff.extend(mesh::INDICES.to_vec());
        Self(buff)
    }
}

render_pass.set_index_buffer(
	index_buffer
		.0
		.buffer()
		.expect("buffer was not set")
		.slice(..),
	0,
	wgpu::IndexFormat::Uint32,
);
```

And now we have our pentagon 

![[pink_pentagon.png]]