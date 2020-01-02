use gdnative::*;

pub unsafe fn connect_signal(emitter: &mut Node, subscriber: &Node, signal: &str, callback: &str) {
    let object = &subscriber.to_object();
    emitter
        .connect(
            GodotString::from_str(signal),
            Some(*object),
            GodotString::from_str(callback),
            VariantArray::new(),
            0,
        )
        .unwrap();
}

pub unsafe fn get_node<T: GodotObject>(tree: &Node, pth: &str) -> Option<T> {
    return tree.get_node(NodePath::from_str(pth)).unwrap().cast::<T>();
}

pub fn load_scene(path: &str) -> Option<PackedScene> {
    let scene = ResourceLoader::godot_singleton().load(
        GodotString::from_str(path), // could also use path.into() here
        GodotString::from_str("PackedScene"),
        false,
    );

    scene.and_then(|s| s.cast::<PackedScene>())
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstancingErrs {
    CouldNotMakeInstance,
    RootClassInvalid(String),
}

pub unsafe fn instance_scene<Root>(scene: &PackedScene) -> Result<Root, InstancingErrs>
where
    Root: gdnative::GodotObject,
{
    let inst_option = scene.instance(0); // 0 - GEN_EDIT_STATE_DISABLED

    if let Some(instance) = inst_option {
        if let Some(instance_root) = instance.cast::<Root>() {
            Ok(instance_root)
        } else {
            Err(InstancingErrs::RootClassInvalid(
                instance.get_name().to_string(),
            ))
        }
    } else {
        Err(InstancingErrs::CouldNotMakeInstance)
    }
}
