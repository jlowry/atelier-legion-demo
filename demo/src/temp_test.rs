use crate::asset_manager::AssetManager;

use atelier_loader::{
    asset_uuid,
    handle::{AssetHandle, Handle},
    rpc_loader::RpcLoader,
    LoadStatus, Loader,
};
use std::collections::HashMap;
use legion::prelude::*;
use crate::clone_merge::CloneMergeImpl;
use crate::components::PositionReference;
use crate::components::Position2DComponentDefinition;
use crate::components::Position2DComponent;

use legion::storage::ComponentTypeId;
use prefab_format::ComponentTypeUuid;
use legion_prefab::ComponentRegistration;
use crate::prefab::PrefabAsset;

pub fn temp_force_load_asset(asset_manager: &mut AssetManager) {
    // Demonstrate loading a component as an asset (probably won't do this in practice)
    {
        let handle = asset_manager
            .loader()
            .add_ref(asset_uuid!("df3a8294-ffce-4ecc-81ad-a96867aa3f8a"));
        let handle =
            Handle::<Position2DComponentDefinition>::new(asset_manager.tx().clone(), handle);
        loop {
            asset_manager.update();
            if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(asset_manager.loader()) {
                let custom_asset: &Position2DComponentDefinition = handle
                    .asset(asset_manager.storage())
                    .expect("failed to get asset");
                log::info!("Loaded a component {:?}", custom_asset);
                break;
            }
        }
    }

    // Create the component registry
    let registered_components = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
            comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone()), reg.clone())),
        );

        component_types
    };

    // Demonstrate loading a prefab
    {
        //
        // Fetch the prefab data
        //
        let handle = asset_manager
            .loader()
            .add_ref(asset_uuid!("49a78d30-0590-4511-9178-302a17f00882"));
        let handle = Handle::<PrefabAsset>::new(asset_manager.tx().clone(), handle);
        loop {
            asset_manager.update();
            if let LoadStatus::Loaded = handle.load_status::<RpcLoader>(asset_manager.loader()) {
                break;
            }
        }

        let prefab_asset: &PrefabAsset = handle.asset(asset_manager.storage()).unwrap();

        //
        // Print legion contents to prove that it worked
        //
        println!("GAME: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDefinition>>::query();
        for pos in query.iter_immutable(&prefab_asset.prefab.world) {
            println!("position: {:?}", pos);
        }
        println!("GAME: done iterating positions");
        println!("GAME: iterating entities");
        for (entity_uuid, entity_id) in &prefab_asset.prefab.prefab_meta.entities {
            println!(
                "GAME: entity {} maps to {:?}",
                uuid::Uuid::from_bytes(*entity_uuid),
                entity_id
            );
        }
        println!("GAME: done iterating entities");

        let universe = Universe::new();
        let mut world = universe.create_world();

        println!("--- CLONE MERGE 1 ---");
        println!("This test just clones Position2DComponentDefinition");
        let clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
        world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDefinition>>::query();
        for (e, pos_def) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        println!("MERGED: done iterating positions");

        println!("--- CLONE MERGE 2 ---");
        println!("This test transforms Position2DComponentDefinition into Position2DComponent");
        let mut clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
        //clone_merge_impl.add_mapping_into::<Position2DComponentDefinition, Position2DComponent>();

        clone_merge_impl.add_mapping::<Position2DComponentDefinition, Position2DComponent, _>(
            |_resources, _entities, from, into| {
                for (f, t) in from.iter().zip(into) {
                    *t = Position2DComponent {
                        position: f.position,
                    };
                }
            },
        );

        world.clone_merge(&prefab_asset.prefab.world, &clone_merge_impl, None, None);

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDefinition>>::query();
        for (e, pos_def) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        println!("MERGED: done iterating positions");

        println!("--- CLONE MERGE 3 ---");
        println!("This test demonstrates replacing existing entities rather than making new ones");
        let mut clone_merge_impl = CloneMergeImpl::new(registered_components.clone());
        clone_merge_impl.add_mapping_into::<Position2DComponentDefinition, Position2DComponent>();

        // Get a list of entities in the prefab
        let mut prefab_entities = vec![];
        let query = <legion::prelude::Read<Position2DComponentDefinition>>::query();
        for (e, _) in query.iter_entities_immutable(&prefab_asset.prefab.world) {
            prefab_entities.push(e);
        }

        // Get a list of entities in the world
        let mut world_entities = vec![];
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, _) in query.iter_entities_immutable(&world) {
            world_entities.push(e);
        }

        // Create a hashmap to map them 1:1
        let mut mappings = HashMap::new();
        for (k, v) in prefab_entities.iter().zip(world_entities) {
            mappings.insert(*k, v);
        }

        println!("mappings: {:#?}", mappings);
        world.clone_merge(
            &prefab_asset.prefab.world,
            &clone_merge_impl,
            Some(&mappings),
            None,
        );

        println!("MERGED: iterate positions");
        let query = <legion::prelude::Read<Position2DComponentDefinition>>::query();
        for (e, pos_def) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position_def: {:?}", e, pos_def);
        }
        let query = <legion::prelude::Read<Position2DComponent>>::query();
        for (e, pos) in query.iter_entities_immutable(&world) {
            println!("entity: {:?} position: {:?}", e, pos);
        }
        let query = <legion::prelude::Read<PositionReference>>::query();
        for (e, pos_ref) in query.iter_entities_immutable(&world) {
            let ref_component: &Position2DComponentDefinition =
                pos_ref.handle.asset(asset_manager.storage()).unwrap();
            println!(
                "entity: {:?} position_ref: {:?} ({:?})",
                e, pos_ref, ref_component
            );
        }
        println!("MERGED: done iterating positions");
    }
}

pub fn temp_force_prefab_cook(asset_manager: &mut AssetManager) {
    // Create the component registry
    let registered_components = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeId, ComponentRegistration> = HashMap::from_iter(
            comp_registrations.map(|reg| (ComponentTypeId(reg.ty().clone()), reg.clone())),
        );

        component_types
    };

    // Create the component registry
    let registered_components_by_uuid = {
        let comp_registrations = legion_prefab::iter_component_registrations();
        use std::iter::FromIterator;
        let component_types: HashMap<ComponentTypeUuid, ComponentRegistration> =
            HashMap::from_iter(comp_registrations.map(|reg| (reg.uuid().clone(), reg.clone())));

        component_types
    };

    let prefab_asset_id = asset_uuid!("5fd8256d-db36-4fe2-8211-c7b3446e1927");

    crate::prefab_cooking::cook_prefab(
        asset_manager,
        &registered_components,
        &registered_components_by_uuid,
        prefab_asset_id,
    );
}