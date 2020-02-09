use ncollide2d::world::CollisionWorld;
use legion::prelude::*;
use std::marker::PhantomData;
use std::sync::Arc;
use crate::resources::{OpenedPrefabState, EditorStateResource};

const EDITOR_SELECTION_WORLD_MARGIN: f32 = 0.02;

/// Any selectable component must implement this trait
pub trait EditorSelectable: legion::storage::Component {
    /// When called, the implementation is expected to place shapes into the collision world
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        resources: &Resources,
        opened_prefab: &OpenedPrefabState,
        world: &World,
        entity: Entity,
    );
}

/// A trait object which allows dynamic dispatch into the selection implementation
trait RegisteredEditorSelectableT: Send + Sync {
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        resources: &Resources,
        opened_prefab: &OpenedPrefabState,
        world: &World,
    );
}

/// Implements the RegisteredEditorSelectableT trait object with code that can call
/// create_editor_selection_world on T
#[derive(Default)]
struct RegisteredEditorSelectable<T> {
    phantom_data: PhantomData<T>,
}

impl<T> RegisteredEditorSelectable<T>
where
    T: EditorSelectable,
{
    fn new() -> Self {
        RegisteredEditorSelectable {
            phantom_data: Default::default(),
        }
    }
}

impl<T> RegisteredEditorSelectableT for RegisteredEditorSelectable<T>
where
    T: EditorSelectable,
{
    fn create_editor_selection_world(
        &self,
        collision_world: &mut CollisionWorld<f32, Entity>,
        resources: &Resources,
        opened_prefab: &OpenedPrefabState,
        world: &World,
    ) {
        let query = <Read<T>>::query();
        for (entity, t) in query.iter_entities(world) {
            t.create_editor_selection_world(
                collision_world,
                resources,
                opened_prefab,
                world,
                entity,
            );
        }
    }
}

#[derive(Default)]
pub struct EditorSelectableRegistry {
    registered: Vec<Box<dyn RegisteredEditorSelectableT>>,
}

impl EditorSelectableRegistry {
    /// Adds a type to the registry, which allows components of these types to receive a callback
    /// to insert shapes into the collision world used for selection
    pub fn register<T: EditorSelectable>(&mut self) {
        self.registered
            .push(Box::new(RegisteredEditorSelectable::<T>::new()));
    }

    /// Produces a collision world that includes shapes associated with entities
    pub fn create_editor_selection_world(
        &self,
        resources: &Resources,
        world: &World,
    ) -> CollisionWorld<f32, Entity> {
        let mut collision_world = CollisionWorld::<f32, Entity>::new(EDITOR_SELECTION_WORLD_MARGIN);

        if let Some(opened_prefab) = resources
            .get::<EditorStateResource>()
            .unwrap()
            .opened_prefab()
            .clone()
        {
            for r in &self.registered {
                r.create_editor_selection_world(
                    &mut collision_world,
                    resources,
                    &*opened_prefab,
                    &world,
                );
            }
        }

        collision_world
    }
}
