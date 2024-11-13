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
    uv_scale: Some(Vec2::new(8., 8.))
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
    acg_path: Res<AmbientCGPath>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cylinder::new(200.0, 0.1)),
            material: EXAMPLE_000.load(acg_path.clone(), asset_server, &mut materials),
            transform: Transform::from_xyz(0.0, -0.05, 0.0),
            ..default()
        },
    ));
}
```