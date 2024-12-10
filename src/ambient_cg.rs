use core::fmt;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};

use bevy::asset::io::file::FileAssetReader;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use image::{DynamicImage, GenericImageView, ImageReader, RgbImage};

pub struct AmbientCGPlugin {
    pub config: AmbientCGConfig
}

static MATERIALS_PATH: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new("materials".to_string()));
static RESOLUTION_NEGOTIATION: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(true));

impl Default for AmbientCGPlugin {
    fn default() -> Self {
        Self {
            config: AmbientCGConfig {
                materials_path: MATERIALS_PATH.lock().unwrap().to_owned(),
                resolution_negotiation: *RESOLUTION_NEGOTIATION.lock().unwrap()}
        }
    }
}

impl Plugin for AmbientCGPlugin {
    fn build(&self, app: &mut App) {
        *MATERIALS_PATH.lock().unwrap() = self.config.materials_path.to_owned();
        *RESOLUTION_NEGOTIATION.lock().unwrap() = self.config.resolution_negotiation;
        app
            .insert_resource::<AmbientCGConfig>(self.config.to_owned());
    }
}

#[derive(Clone, Debug, Resource)]
pub struct AmbientCGConfig {
    pub materials_path: String,
    pub resolution_negotiation: bool
}

#[allow(dead_code)]
#[derive(Clone, Default)]
pub enum AmbientCGResolution {
    #[default]
    OneK,
    TwoK,
    FourK,
    EightK,
    TwelveK,
    SixteenK,
}

#[allow(dead_code)]
impl AmbientCGResolution {
    pub fn next_smaller(&self) -> Result<Self, AmbientCGImportError> {
        match &self {
            Self::OneK => Err(AmbientCGImportError(AmbientCGErrorType::NoSmallerRes)),
            Self::TwoK => Ok(Self::OneK),
            Self::FourK => Ok(Self::TwoK),
            Self::EightK => Ok(Self::FourK),
            Self::TwelveK => Ok(Self::EightK),
            Self::SixteenK => Ok(Self::TwelveK),
        }
    }
}

impl std::fmt::Display for AmbientCGResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match *self {
            Self::OneK => "1K",
            Self::TwoK => "2K",
            Self::FourK => "4K",
            Self::EightK => "8K",
            Self::TwelveK => "12K",
            Self::SixteenK => "16K",
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug)]
pub struct AmbientCGImportError(AmbientCGErrorType);

#[allow(dead_code)]
#[derive(Debug)]
enum AmbientCGErrorType {
    NoSmallerRes,
    NotFound,
}

impl fmt::Display for AmbientCGImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl Error for AmbientCGImportError {
    fn description(&self) -> &str {
        match self.0 {
            AmbientCGErrorType::NoSmallerRes => "Could not find a smaller texture size than missing requested resolution",
            AmbientCGErrorType::NotFound => "Material not found in assets folder"
        }
    }
}

#[derive(Clone, Default, Resource)]
pub struct AmbientCGMaterial<'a> {
    pub name: &'a str,
    pub resolution: AmbientCGResolution,
    pub subfolder: Option<&'a str>,
    pub uv_scale: Option<Vec2>
}

impl<'a> AmbientCGMaterial<'a> {
    fn negotiate_resolution(self, materials_path: &PathBuf) ->  Result<AmbientCGMaterial<'a>, AmbientCGImportError> {
        let constructed_material_name = format!("{}_{}-JPG", self.name, self.resolution);
        let mut resource_path = materials_path.clone();
        resource_path.push(constructed_material_name);
        if !&absolute_resource_path(&resource_path).exists() {
            let resolution = match self.resolution.next_smaller() {
                Ok(resolution) => resolution,
                Err(error) => return Err(error)
            };
            return AmbientCGMaterial::negotiate_resolution(Self {
                name: self.name,
                resolution,
                subfolder: self.subfolder,
                uv_scale: self.uv_scale
            }, materials_path)
        }
        let ambient_cgmaterial = self.clone();
        Ok(ambient_cgmaterial)
    }
    pub fn load(
        &self,
        asset_server: &Res<'_, AssetServer>,
        materials: &mut ResMut<'_, Assets<StandardMaterial>>,
    ) -> Handle<StandardMaterial> {
        if let Some(uv_scale) = self.uv_scale {
            return self.load_with_uv_scale(asset_server, materials, uv_scale);
        }
        self.load_without_uv_scale(asset_server, materials)
    }
    pub fn load_without_uv_scale(
        &self,
        asset_server: &Res<'_, AssetServer>,
        materials: &mut ResMut<'_, Assets<StandardMaterial>>
    ) -> Handle<StandardMaterial> {
        self.load_with_uv_scale(asset_server, materials, Vec2::ZERO)
    }
    pub fn load_with_uv_scale(
        &self,
        asset_server: &Res<'_, AssetServer>,
        materials: &mut ResMut<'_, Assets<StandardMaterial>>,
        uv_scale: Vec2
    ) -> Handle<StandardMaterial> {
        let mut material_path =PathBuf::from_str(&MATERIALS_PATH.lock().unwrap()).unwrap();

        if let Some(subfolder) = &self.subfolder {
            material_path.push(subfolder);
        }

        let mut ambient_cg_material = self.clone();
        if *RESOLUTION_NEGOTIATION.lock().unwrap() {
            ambient_cg_material = match self.clone().negotiate_resolution(&material_path) {
                Ok(ambient_cg_material) => {
                    let ambient_cgmaterial = ambient_cg_material.to_owned();
                    ambient_cgmaterial
                },
                Err(err) => panic!("{}", err)
            }
        }

        let constructed_material_name = format!("{}_{}-JPG", ambient_cg_material.name, ambient_cg_material.resolution);
        material_path.push(constructed_material_name.clone());
        
        let occlusion_path = material_path.join(constructed_material_name.clone() + "_AmbientOcclusion").with_extension("jpg");
        let base_color_path = material_path.join(constructed_material_name.clone() + "_Color").with_extension("jpg");
        let thickness_path = material_path.join(constructed_material_name.clone() + "_Displacement").with_extension("jpg");
        let metallic_texture_path = material_path.join(constructed_material_name.clone() + "_Metalness").with_extension("jpg");
        let normal_map_path = material_path.join(constructed_material_name.clone() + "_NormalGL").with_extension("jpg");
        let roughness_texture_path = material_path.join(constructed_material_name.clone() + "_Roughness").with_extension("jpg");

        let repeat_texture = 
        |s: &mut _| {
            *s = ImageLoaderSettings {
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    // rewriting mode to repeat image,
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                }),
                ..default()
            }
        };

        let occlusion_texture_exists = Path::exists(&absolute_resource_path(&occlusion_path));
        let base_color_texture_exists = Path::exists(&absolute_resource_path(&base_color_path));
        let thickness_texture_exists = Path::exists(&absolute_resource_path(&thickness_path));
        let metallic_texture_exists = Path::exists(&absolute_resource_path(&metallic_texture_path));
        let normal_map_texture_exists = Path::exists(&absolute_resource_path(&normal_map_path));
        let roughness_texture_exists = Path::exists(&absolute_resource_path(&roughness_texture_path));
        
        let occlusion_texture: Option<Handle<Image>> = if occlusion_texture_exists {Some(asset_server.load_with_settings(occlusion_path, repeat_texture))} else { None };
        let base_color_texture: Option<Handle<Image>> = if base_color_texture_exists {Some(asset_server.load_with_settings(base_color_path, repeat_texture))} else { None };
        let thickness_texture: Option<Handle<Image>> = if thickness_texture_exists {Some(asset_server.load_with_settings(thickness_path, repeat_texture))} else { None };
        let normal_map_texture: Option<Handle<Image>> = if normal_map_texture_exists {Some(asset_server.load_with_settings(normal_map_path, repeat_texture))} else { None };

        let mut metallic_roughness_texture = None;
        if metallic_texture_exists && roughness_texture_exists {
            metallic_roughness_texture = Some(asset_server.add(
                create_roughness_metallic_image(
                    absolute_resource_path(&metallic_texture_path),
                    absolute_resource_path(&roughness_texture_path)
                )));
        } else if metallic_texture_exists {
            metallic_roughness_texture = Some(asset_server.load_with_settings(metallic_texture_path, repeat_texture));
        } else if roughness_texture_exists {
            metallic_roughness_texture = Some(asset_server.load_with_settings(roughness_texture_path, repeat_texture));
        }

        let material = StandardMaterial {
            base_color_texture,
            metallic_roughness_texture,
            metallic: 1.0,
            normal_map_texture,
            occlusion_texture,
            perceptual_roughness: 1.0,
            thickness_texture,
            uv_transform: (|| {
                if uv_scale == Vec2::ZERO {
                    return Affine2::default();
                }
                Affine2::from_scale(uv_scale)
            })(),
            ..default()
        };
        materials.add(material)
    }
}

fn absolute_resource_path(p: &PathBuf) -> PathBuf {
    let mut path = FileAssetReader::get_base_path();
    let p = p.clone().into_os_string();
    let s = OsStr::new("assets");
    path.push(s);
    path.push(p);
    path
}

fn create_roughness_metallic_image(roughness_path: PathBuf, metallic_path: PathBuf) -> Image {
    let roughness = load_grayscale_image(&roughness_path);
    let metallic = load_grayscale_image(&metallic_path);

    assert_eq!(roughness.width(), metallic.width(), "Images must have the same width");
    assert_eq!(roughness.height(), metallic.height(), "Images must have the same height");

    let (width, height) = (roughness.width(), roughness.height());
    
    let mut metallic_roughness = RgbImage::new(width, height);

    for (x, y, pixel) in metallic_roughness.enumerate_pixels_mut() {
        let roughness = roughness.get_pixel(x, y)[0];
        let metallic = metallic.get_pixel(x, y)[0];

        // Set the new pixel's color (R = 0, G = roughness, B = metallic)
        let color = [0, roughness, metallic];

        pixel.0 = color;
    }

    Image::from_dynamic(
        DynamicImage::ImageRgb8(metallic_roughness),
        false,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD
    )
}

fn load_grayscale_image(path: &PathBuf) -> DynamicImage {
    let image = ImageReader::open(path).expect("Could not load image").decode();
    image.expect("Could not determine file encoding").grayscale()
}