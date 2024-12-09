### bevy_ambient_cg
---
## Summary
This plugin allows you to easily import Ambient CG materials into Bevy with only a few lines of code.

Roughness/Metallic maps are automatically constructed with roughness data and metallic data going in the green and blue channels respectively of a generated map during runtime. No manual file conversions!

As of now, only JPEG format images are implemented and will require enabling the bevy jpg feature.

```
cargo add bevy -F jpg
```

## Examples
Constructing an ambient CG material resource
```Rust
pub const EXAMPLE_000: AmbientCGMaterial = AmbientCGMaterial {
    name: "Example000",
    subfolder: Some("some/path/to/resource"),
    resolution: AmbientCGResolution::OneK,
    // this is the uv scale you want to render at, materials are generated to repeat
    // if uv_scale is None asset server will use default Affine value when loading
    uv_scale: Some(Vec2::new(8., 8.))
};

pub const EXAMPLE_001: AmbientCGMaterial = AmbientCGMaterial {
    name: "Example001",
    subfolder: Some("some/path/to/resource"),
    // Resolution will auto negotiate to a smaller resolution if 16K is not found.
    // This will allow you to selectively bundle textures and not have to determine resolution that is currently loaded if so desired
    resolution: AmbientCGResolution::SixteenK,
    uv_scale: None,
};
```
---
Initializing plugin
```Rust
fn main() {
    app.add_plugins(DefaultPlugins)
        // by default this will look for materials in assets/materials
        .add_plugins(AmbientCGPlugin::default())
        .run()
}
```
---
Load a material and apply to mesh
```Rust
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cylinder::new(200.0, 0.1)),
            material: EXAMPLE_000.load(&asset_server, &mut materials),
            transform: Transform::from_xyz(0.0, -0.05, 0.0),
            ..default()
        },
    ));

    // This will override the UV Scale defined in the const
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cylinder::new(200.0, 0.1)),
            // Here we define UV scale on the fly to override the value from defined AmbientCGMaterial
            material: EXAMPLE_001.load_with_uv_scale(&asset_server, &mut materials, Vec2::(2.0, 2.0)),
            transform: Transform::from_xyz(0.0, -0.05, 0.0),
            ..default()
        },
    ));
}
```