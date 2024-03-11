use bevy::{
    prelude::*,
    render::{
        render_graph::{RenderGraph, RenderGraphApp},
        RenderApp,
    },
};

use self::node::SurfaceNode;

pub struct ApplierPlugin;

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

mod node {
    use bevy::{
        ecs::world::FromWorld,
        render::{
            render_graph::Node,
            render_resource::{LoadOp, Operations, RenderPassColorAttachment, StoreOp},
            view::ExtractedWindows,
        },
    };
    use wgpu::{Color, RenderPassDescriptor};

    use super::graph::ApplierSubgraph;

    pub struct SurfaceNode;

    impl Node for SurfaceNode {
        fn run<'w>(
            &self,
            _graph: &mut bevy::render::render_graph::RenderGraphContext,
            render_context: &mut bevy::render::renderer::RenderContext<'w>,
            world: &'w bevy::prelude::World,
        ) -> Result<(), bevy::render::render_graph::NodeRunError> {
            let windows = world.resource::<ExtractedWindows>();
            for window in windows.values() {
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
                    let mut _render_pass =
                        render_context.begin_tracked_render_pass(RenderPassDescriptor {
                            label: Some("applied_pass"),
                            color_attachments: &[color_attachment],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
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

impl Plugin for ApplierPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            render_graph.add_node(graph::ApplierNode::ExecuteNode, node::ExecuteNode);

            render_app
                .add_render_sub_graph(graph::ApplierSubgraph)
                .add_render_graph_node::<SurfaceNode>(
                    graph::ApplierSubgraph,
                    graph::ApplierNode::SurfaceNode,
                );
        }
    }
}
