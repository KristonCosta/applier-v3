use bevy::{
    asset::load_internal_asset,
    ecs::system::{StaticSystemParam, SystemParamItem},
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_graph::{RenderGraph, RenderGraphApp},
        render_resource::{
            binding_types::uniform_buffer, AsBindGroup, BindGroup, BindGroupEntries,
            BindGroupLayout, BindGroupLayoutEntries, DynamicUniformBuffer, RawBufferVec,
            ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, Render, RenderApp, RenderSet,
    },
};
use camera::CameraUniform;
use cgmath::{Point3, Vector3};
use wgpu::BufferUsages;

use crate::plugin::pipeline::{ApplierPipeline, APPLIER_SHADER_HANDLE};

use self::{
    material::{ApplierMaterial, PreparedApplierMaterial},
    node::SurfaceNode,
};

pub struct ApplierPlugin;

mod camera {
    use bevy::{prelude::*, render::render_resource::ShaderType};
    use bitmask_enum::bitmask;
    use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Vector3, Vector4};

    #[rustfmt::skip]
    const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.5,
        0.0, 0.0, 0.0, 1.0,
    );

    #[derive(Resource, Clone, Debug)]
    pub struct Camera {
        pub eye: Point3<f32>,
        pub target: Point3<f32>,
        pub up: Vector3<f32>,
        pub aspect: f32,
        pub fovy: f32,
        pub znear: f32,
        pub zfar: f32,
    }

    pub struct Projection(Matrix4<f32>);

    fn vector_to_vec(from: Vector4<f32>) -> Vec4 {
        Vec4::new(from.x, from.y, from.z, from.w)
    }

    impl Into<Mat4> for Projection {
        fn into(self) -> Mat4 {
            let inner = self.0;
            Mat4::from_cols(
                vector_to_vec(inner.x),
                vector_to_vec(inner.y),
                vector_to_vec(inner.z),
                vector_to_vec(inner.w),
            )
        }
    }
    impl Camera {
        pub fn build_view_projection_matrix(&self) -> Projection {
            let view = Matrix4::look_at_rh(self.eye, self.target, self.up);
            let proj = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

            Projection(OPENGL_TO_WGPU_MATRIX * proj * view)
        }
    }

    #[repr(C)]
    #[derive(Debug, Clone, ShaderType)]
    pub struct CameraUniform {
        pub view_proj: Mat4,
    }

    #[bitmask(u8)]
    pub enum CameraDirection {
        Forward = 0b00000001,
        Backward = 0b00000010,
        Left = 0b00000100,
        Right = 0b00001000,
        Up = 0b00010000,
        Down = 0b00100000,
    }

    pub struct CameraPlugin;

    impl Plugin for CameraPlugin {
        fn build(&self, app: &mut App) {
            app.add_event::<CameraEvent>()
                .add_systems(Update, (handle_camera_input, process_camera_events));
        }
    }

    #[derive(Event)]
    pub enum CameraEvent {
        // The move camera should have a bit mask that lets us define forwaard, backward, left, right, up, down
        MoveCamera(CameraDirection),
    }

    const CAMERA_SPEED: f32 = 0.2;

    fn process_camera_events(mut events: EventReader<CameraEvent>, mut camera: ResMut<Camera>) {
        for event in events.read() {
            match event {
                CameraEvent::MoveCamera(direction) => {
                    let forward = camera.target - camera.eye;
                    let forward_norm = forward.normalize();

                    if direction.contains(CameraDirection::Forward) {
                        camera.eye += forward_norm * CAMERA_SPEED;
                    }
                    if direction.contains(CameraDirection::Backward) {
                        camera.eye -= forward_norm * CAMERA_SPEED;
                    }

                    let right = forward_norm.cross(camera.up);

                    let forward = camera.target - camera.eye;
                    let forward_mag = forward.magnitude();

                    if direction.contains(CameraDirection::Right) {
                        camera.eye = camera.target
                            - (forward + right * CAMERA_SPEED).normalize() * forward_mag;
                    }

                    if direction.contains(CameraDirection::Left) {
                        camera.eye = camera.target
                            - (forward - right * CAMERA_SPEED).normalize() * forward_mag;
                    }
                }
            }
        }
    }

    fn handle_camera_input(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut camera_events: EventWriter<CameraEvent>,
    ) {
        let mut direction = CameraDirection::none();

        if keyboard_input.pressed(KeyCode::KeyW) {
            direction |= CameraDirection::Forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction |= CameraDirection::Backward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction |= CameraDirection::Left;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction |= CameraDirection::Right;
        }
        if direction != CameraDirection::none() {
            camera_events.send(CameraEvent::MoveCamera(direction));
        }
    }
}

mod graph {
    use bevy::render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct ApplierSubgraph;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum ApplierNode {
        ExecuteNode,
        SurfaceNode,
    }
}

mod mesh {
    use std::mem;

    use bevy::render::render_resource::{ShaderType, VertexBufferLayout};

    use wgpu::{BufferAddress, VertexStepMode};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, ShaderType)]
    pub struct Vertex {
        position: [f32; 3],
        tex_coords: [f32; 2],
    }

    pub const VERTICES: &[Vertex] = &[
        Vertex {
            position: [-0.0868241, 0.49240386, 0.0],
            tex_coords: [0.4131759, 0.00759614],
        },
        Vertex {
            position: [-0.49513406, 0.06958647, 0.0],
            tex_coords: [0.0048659444, 0.43041354],
        },
        Vertex {
            position: [-0.21918549, -0.44939706, 0.0],
            tex_coords: [0.28081453, 0.949397],
        },
        Vertex {
            position: [0.35966998, -0.3473291, 0.0],
            tex_coords: [0.85967, 0.84732914],
        },
        Vertex {
            position: [0.44147372, 0.2347359, 0.0],
            tex_coords: [0.9414737, 0.2652641],
        },
    ];

    pub const INDICES: &[u32] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];

    impl Vertex {
        pub fn desc() -> VertexBufferLayout {
            VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2, // NEW!
                    },
                ],
            }
        }
    }
}

mod node {
    use bevy::{
        ecs::world::FromWorld,
        render::{
            render_graph::Node,
            render_resource::{
                LoadOp, Operations, PipelineCache, RenderPassColorAttachment, StoreOp,
            },
            view::ExtractedWindows,
        },
    };
    use wgpu::{Color, RenderPassDescriptor};

    use super::{
        graph::ApplierSubgraph, material::PreparedApplierMaterial, pipeline::ApplierPipeline,
        CameraBuffer, IndexBuffer, MousePosition, VertexBuffer,
    };

    pub struct SurfaceNode;

    impl Node for SurfaceNode {
        fn run<'w>(
            &self,
            _graph: &mut bevy::render::render_graph::RenderGraphContext,
            render_context: &mut bevy::render::renderer::RenderContext<'w>,
            world: &'w bevy::prelude::World,
        ) -> Result<(), bevy::render::render_graph::NodeRunError> {
            let windows = world.resource::<ExtractedWindows>();
            let mouse_position = world.resource::<MousePosition>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let applier_pipeline = world.resource::<ApplierPipeline>();
            let vertex_buffer = world.resource::<VertexBuffer>();
            let index_buffer = world.resource::<IndexBuffer>();
            let bind_group = world.resource::<PreparedApplierMaterial>();
            let camera_bind_group = world
                .resource::<CameraBuffer>()
                .bind_group
                .as_ref()
                .unwrap();

            for window in windows.values() {
                if let Some(view) = window.swap_chain_texture_view.as_ref() {
                    let color_attachment = Some(RenderPassColorAttachment {
                        view: view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: (mouse_position.0 as f64 / window.physical_width as f64),
                                g: (mouse_position.1 as f64 / window.physical_height as f64),
                                b: ((window.physical_width as f64 - mouse_position.0 as f64)
                                    / window.physical_width as f64),
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    });
                    let mut render_pass =
                        render_context.begin_tracked_render_pass(RenderPassDescriptor {
                            label: Some("applied_pass"),
                            color_attachments: &[color_attachment],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                    if let Some(pipeline) = pipeline_cache.get_render_pipeline(applier_pipeline.id)
                    {
                        render_pass.set_render_pipeline(pipeline);
                        render_pass.set_bind_group(0, &bind_group.bind_group, &[]);
                        render_pass.set_bind_group(1, camera_bind_group, &[]);
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
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..index_buffer.0.len() as u32, 0, 0..1)
                    }
                }
            }
            Ok(())
        }
    }

    impl FromWorld for SurfaceNode {
        fn from_world(_world: &mut bevy::prelude::World) -> Self {
            SurfaceNode
        }
    }

    pub struct ExecuteNode;

    impl Node for ExecuteNode {
        fn run<'w>(
            &self,
            graph: &mut bevy::render::render_graph::RenderGraphContext,
            _render_context: &mut bevy::render::renderer::RenderContext<'w>,
            _world: &'w bevy::prelude::World,
        ) -> Result<(), bevy::render::render_graph::NodeRunError> {
            graph.run_sub_graph(ApplierSubgraph, vec![], None)?;
            Ok(())
        }
    }
}

mod material {
    use bevy::{
        asset::{AssetServer, Handle},
        ecs::{system::Resource, world::FromWorld},
        render::render_resource::{AsBindGroup, BindGroup, OwnedBindingResource},
    };
    use bevy_internal::image::Image;

    #[derive(AsBindGroup, Resource)]
    pub struct ApplierMaterial {
        #[texture(0)]
        #[sampler(1)]
        pub image: Handle<Image>,
    }

    impl FromWorld for ApplierMaterial {
        fn from_world(world: &mut bevy::prelude::World) -> Self {
            let asset_server = world.resource::<AssetServer>();
            let handle = asset_server.load("tree.png");
            Self { image: handle }
        }
    }

    #[derive(Resource)]
    pub struct PreparedApplierMaterial {
        pub _bindings: Vec<(u32, OwnedBindingResource)>,
        pub bind_group: BindGroup,
    }
}

mod pipeline {
    use bevy::{
        asset::Handle,
        ecs::{system::Resource, world::FromWorld},
        render::{
            render_resource::{
                AsBindGroup, BindGroupLayout, CachedRenderPipelineId, FragmentState, PipelineCache,
                RenderPipelineDescriptor, Shader, VertexState,
            },
            renderer::RenderDevice,
        },
    };
    use wgpu::{
        BlendState, ColorTargetState, ColorWrites, Face, FrontFace, MultisampleState, PolygonMode,
        PrimitiveState, PrimitiveTopology, TextureFormat,
    };

    use super::{material::ApplierMaterial, mesh::Vertex, CameraBuffer};

    pub const APPLIER_SHADER_HANDLE: Handle<Shader> =
        Handle::weak_from_u128(154484490495509739857733487233335592041);

    #[derive(Resource)]
    pub struct ApplierPipeline {
        pub id: CachedRenderPipelineId,
        pub material_layout: BindGroupLayout,
    }

    impl FromWorld for ApplierPipeline {
        fn from_world(world: &mut bevy::prelude::World) -> Self {
            let mut camera = world.remove_resource::<CameraBuffer>().unwrap();

            let render_device = world.resource::<RenderDevice>();
            let material_layout = ApplierMaterial::bind_group_layout(render_device);

            camera.init_bind_group_layout(render_device);
            world.insert_resource(camera);
            let camera = world.resource::<CameraBuffer>();
            let descriptor = RenderPipelineDescriptor {
                vertex: VertexState {
                    shader: APPLIER_SHADER_HANDLE,
                    entry_point: "vs_main".into(),
                    shader_defs: vec![],
                    buffers: vec![Vertex::desc()],
                },
                fragment: Some(FragmentState {
                    shader: APPLIER_SHADER_HANDLE,
                    shader_defs: vec![],
                    entry_point: "fs_main".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Bgra8UnormSrgb,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                layout: vec![
                    material_layout.clone(),
                    camera.layout.as_ref().unwrap().clone(),
                ],
                push_constant_ranges: Vec::new(),
                primitive: PrimitiveState {
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                label: Some("applier_pipeline".into()),
                zero_initialize_workgroup_memory: true,
            };
            let cache = world.resource_mut::<PipelineCache>();
            let id = cache.queue_render_pipeline(descriptor);

            Self {
                id,
                material_layout,
            }
        }
    }
}

impl Plugin for ApplierPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            APPLIER_SHADER_HANDLE,
            "shaders.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(camera::CameraPlugin)
            .insert_resource(MousePosition(0.0, 0.0))
            .init_resource::<ApplierMaterial>()
            .insert_resource(camera::Camera {
                eye: Point3::new(0.0, 0.0, 1.0),
                target: Point3::new(0.0, 0.0, 0.0),
                up: Vector3::new(0.0, 1.0, 0.0),
                aspect: 1.0,
                fovy: 45.0,
                znear: 0.1,
                zfar: 100.0,
            })
            .add_systems(Update, (cursor_events,));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(MousePosition(0.0, 0.0))
                .init_resource::<VertexBuffer>()
                .init_resource::<IndexBuffer>()
                .init_resource::<CameraBuffer>()
                .add_systems(
                    ExtractSchedule,
                    (extract_mouse_position, extract_material, extract_camera),
                )
                .add_systems(
                    Render,
                    (
                        prepare_buffers.in_set(RenderSet::PrepareResources),
                        prepare_bind_groups.in_set(RenderSet::PrepareResources),
                    ),
                );

            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(graph::ApplierNode::ExecuteNode, node::ExecuteNode);

            render_graph
                .remove_node(CameraDriverLabel)
                .expect("failed to remove camera driver");

            render_app
                .add_render_sub_graph(graph::ApplierSubgraph)
                .add_render_graph_node::<SurfaceNode>(
                    graph::ApplierSubgraph,
                    graph::ApplierNode::SurfaceNode,
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<ApplierPipeline>();
        }
    }
}

#[derive(Resource)]
pub struct CameraBuffer {
    buf: DynamicUniformBuffer<CameraUniform>,
    bind_group: Option<BindGroup>,
    layout: Option<BindGroupLayout>,
}

impl FromWorld for CameraBuffer {
    fn from_world(_world: &mut World) -> Self {
        let buf = DynamicUniformBuffer::default();

        Self {
            buf,
            bind_group: None,
            layout: None,
        }
    }
}

impl CameraBuffer {
    pub fn try_init_bind_group(&mut self, render_device: &RenderDevice) -> bool {
        if let Some(layout) = self.layout.as_ref() {
            self.bind_group = Some(render_device.create_bind_group(
                "Camera bind group",
                layout,
                &BindGroupEntries::single(self.buf.buffer().unwrap().as_entire_buffer_binding()),
            ));
            true
        } else {
            false
        }
    }

    pub fn init_bind_group_layout(&mut self, render_device: &RenderDevice) {
        self.layout = Some(
            render_device.create_bind_group_layout(
                "Camera bind group layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX,
                    (uniform_buffer::<CameraUniform>(false)
                        .visibility(ShaderStages::VERTEX_FRAGMENT),),
                ),
            ),
        );
    }
}

#[derive(Resource)]
pub struct VertexBuffer(RawBufferVec<mesh::Vertex>);

impl FromWorld for VertexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = RawBufferVec::new(BufferUsages::VERTEX);
        buff.extend(mesh::VERTICES.to_vec());
        Self(buff)
    }
}

#[derive(Resource)]
pub struct IndexBuffer(RawBufferVec<u32>);

impl FromWorld for IndexBuffer {
    fn from_world(_world: &mut World) -> Self {
        let mut buff = RawBufferVec::new(BufferUsages::INDEX);
        buff.extend(mesh::INDICES.to_vec());
        Self(buff)
    }
}

fn extract_mouse_position(
    mut mouse_position: ResMut<MousePosition>,
    main_mouse_position: Extract<Res<MousePosition>>,
) {
    mouse_position.0 = main_mouse_position.0;
    mouse_position.1 = main_mouse_position.1;
}

fn extract_material(
    mut commands: Commands,
    extracted_material: Option<Res<ApplierMaterial>>,
    main_material: Extract<Res<ApplierMaterial>>,
) {
    if extracted_material.is_none() {
        commands.insert_resource(ApplierMaterial {
            image: main_material.image.clone(),
        })
    }
}

pub fn extract_camera(
    mut camera_buffer: ResMut<CameraBuffer>,
    main_camera: Extract<Res<camera::Camera>>,
) {
    let view_proj = main_camera.build_view_projection_matrix();
    camera_buffer.buf.clear();
    camera_buffer.buf.push(&CameraUniform {
        view_proj: view_proj.into(),
    });
}

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

fn prepare_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut vertex_buffer: ResMut<VertexBuffer>,
    mut index_buffer: ResMut<IndexBuffer>,
    mut uniform_buffer: ResMut<CameraBuffer>,
) {
    vertex_buffer.0.write_buffer(&render_device, &render_queue);
    index_buffer.0.write_buffer(&render_device, &render_queue);
    uniform_buffer
        .buf
        .write_buffer(&render_device, &render_queue);
}

fn prepare_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    material: Res<ApplierMaterial>,
    mut param: StaticSystemParam<SystemParamItem<'_, '_, <ApplierMaterial as AsBindGroup>::Param>>,
    prepared_material: Option<Res<PreparedApplierMaterial>>,
    pipeline: Res<ApplierPipeline>,
    mut camera: ResMut<CameraBuffer>,
) {
    if prepared_material.is_none() {
        let prepared = material
            .as_bind_group(&pipeline.material_layout, &render_device, &mut param)
            .expect("failed to prepare bind group");

        commands.insert_resource(PreparedApplierMaterial {
            _bindings: prepared.bindings,
            bind_group: prepared.bind_group,
        });
    }
    if camera.bind_group.is_none() {
        camera.try_init_bind_group(&render_device);
    }
}
