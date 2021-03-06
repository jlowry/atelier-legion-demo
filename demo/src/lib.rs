#[macro_use]
extern crate itertools;

extern crate nalgebra as na;
extern crate nalgebra_glm as glm;

use legion::prelude::*;

use std::collections::HashMap;
use legion::storage::ComponentTypeId;
use legion_prefab::ComponentRegistration;
use prefab_format::ComponentTypeUuid;
use atelier_core::asset_uuid;

mod asset_storage;

mod components;
use components::*;

mod resources;
use resources::*;

mod systems;
use systems::*;

mod selection;
use selection::EditorSelectableRegistry;

mod inspect;
use inspect::EditorInspectRegistry;

pub mod math;

mod pipeline;
use pipeline::*;
use std::sync::mpsc::RecvTimeoutError::Timeout;
use std::borrow::BorrowMut;
use nphysics2d::object::RigidBodyDesc;

pub mod daemon;

mod prefab_cooking;

pub mod app;

mod imgui_support;

use legion_transaction::CopyCloneImpl;
use legion_transaction::SpawnCloneImpl;
use legion_transaction::SpawnCloneImplHandlerSet;

pub const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
pub const GRAVITY: f32 = -9.81;

/// Create the asset manager that has all the required types registered
pub fn create_asset_manager() -> AssetResource {
    let mut asset_manager = AssetResource::default();
    asset_manager.add_storage::<PrefabAsset>();
    asset_manager
}

pub fn create_component_registry() -> HashMap<ComponentTypeId, ComponentRegistration> {
    let comp_registrations = legion_prefab::iter_component_registrations();
    use std::iter::FromIterator;
    let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
        comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone(), #[cfg(feature = "ffi")] 0), reg.clone())),
    );

    component_types
}

pub fn create_component_registry_by_uuid() -> HashMap<ComponentTypeUuid, ComponentRegistration> {
    let comp_registrations = legion_prefab::iter_component_registrations();
    use std::iter::FromIterator;
    let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
        HashMap::from_iter(comp_registrations.map(|reg| (reg.uuid().clone(), reg.clone())));

    component_types
}

pub fn create_spawn_clone_impl_handler_set(
) -> SpawnCloneImplHandlerSet {
    let mut handler_set = SpawnCloneImplHandlerSet::new();
    handler_set.add_mapping_into::<DrawSkiaCircleComponentDef, DrawSkiaCircleComponent>();
    handler_set.add_mapping_into::<DrawSkiaBoxComponentDef, DrawSkiaBoxComponent>();
    handler_set.add_mapping::<RigidBodyBallComponentDef, RigidBodyComponent>();
    handler_set.add_mapping::<RigidBodyBoxComponentDef, RigidBodyComponent>();
    handler_set
}

pub fn create_spawn_clone_impl<'a, 'b, 'c>(
    handler_set: &'a SpawnCloneImplHandlerSet,
    component_registry: &'b HashMap<ComponentTypeId, ComponentRegistration>,
    resources: &'c Resources
) -> SpawnCloneImpl<'a, 'b, 'c> {
    SpawnCloneImpl::new(handler_set, component_registry, resources)
}

pub fn create_editor_selection_registry() -> EditorSelectableRegistry {
    let mut registry = EditorSelectableRegistry::default();
    registry.register::<DrawSkiaBoxComponent>();
    registry.register::<DrawSkiaCircleComponent>();
    registry.register_transformed::<RigidBodyBoxComponentDef, RigidBodyComponent>();
    registry.register_transformed::<RigidBodyBallComponentDef, RigidBodyComponent>();
    registry
}

pub fn create_editor_inspector_registry() -> EditorInspectRegistry {
    let mut registry = EditorInspectRegistry::default();
    registry.register::<DrawSkiaCircleComponentDef>();
    registry.register::<DrawSkiaBoxComponentDef>();
    registry.register::<Position2DComponent>();
    registry.register::<UniformScale2DComponent>();
    registry.register::<NonUniformScale2DComponent>();
    registry.register::<Rotation2DComponent>();
    registry.register::<RigidBodyBallComponentDef>();
    registry.register::<RigidBodyBoxComponentDef>();
    registry
}

pub struct DemoApp {
    update_schedules: HashMap<ScheduleCriteria, Schedule>,
    draw_schedules: HashMap<ScheduleCriteria, Schedule>,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // The expected states for which we will generate schedules
        let expected_criteria = vec![
            ScheduleCriteria::new(false, EditorMode::Inactive),
            ScheduleCriteria::new(true, EditorMode::Active),
        ];

        // Populate a lookup for the schedules.. on each update/draw, we will check the current
        // state of the application, create an appropriate ScheduleCriteria, and use it to look
        // up the correct schedule to run
        let mut update_schedules = HashMap::default();
        let mut draw_schedules = HashMap::default();

        for criteria in &expected_criteria {
            update_schedules.insert(criteria.clone(), systems::create_update_schedule(&criteria));
            draw_schedules.insert(criteria.clone(), systems::create_draw_schedule(&criteria));
        }

        DemoApp {
            update_schedules,
            draw_schedules,
        }
    }

    // Determine the current state of the game
    fn get_current_schedule_criteria(resources: &Resources) -> ScheduleCriteria {
        ScheduleCriteria::new(
            resources
                .get::<TimeResource>()
                .unwrap()
                .is_simulation_paused(),
            resources
                .get::<EditorStateResource>()
                .unwrap()
                .editor_mode(),
        )
    }
}

impl app::AppHandler for DemoApp {
    fn init(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        let asset_manager = create_asset_manager();
        let physics = PhysicsResource::new(glam::Vec2::unit_y() * GRAVITY);

        let window_size = resources.get::<InputResource>().unwrap().window_size();

        let mut camera = CameraResource::new(
            glam::Vec2::new(0.0, 1.0),
            crate::GROUND_HALF_EXTENTS_WIDTH * 1.5,
        );
        let viewport = ViewportResource::new(window_size, camera.position, camera.x_half_extents);

        resources.insert(physics);
        resources.insert(FpsTextResource::new());
        resources.insert(asset_manager);
        resources.insert(EditorStateResource::new());
        resources.insert(camera);
        resources.insert(viewport);
        resources.insert(DebugDrawResource::new());
        resources.insert(EditorDrawResource::new());

        let selection_resource =
            EditorSelectionResource::new(create_editor_selection_registry(), resources, world);

        resources.insert(selection_resource);

        // Start the application
        EditorStateResource::open_prefab(
            world,
            resources,
            asset_uuid!("3991506e-ed7e-4bcb-8cfd-3366b31a6439"),
        );
    }

    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        let current_criteria = Self::get_current_schedule_criteria(resources);
        let mut schedule = self.update_schedules.get_mut(&current_criteria).unwrap();
        schedule.execute(world, resources);
    }

    fn draw(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        let current_criteria = Self::get_current_schedule_criteria(resources);
        let mut schedule = self.draw_schedules.get_mut(&current_criteria).unwrap();
        schedule.execute(world, resources);
    }

    fn fatal_error(
        &mut self,
        error: &app::AppError,
    ) {
        log::error!("{}", error);
    }
}
