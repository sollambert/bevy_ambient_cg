use core::fmt;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::str::FromStr;

use bevy::asset::io::file::FileAssetReader;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::texture::*;
use image::{DynamicImage, GenericImageView, ImageReader, RgbImage};

pub struct AmbientCGPlugin {
    base_path: String
}

impl Default for AmbientCGPlugin {
    fn default() -> Self {
        Self {
            base_path: "materials".to_string(),
        }
    }
}

impl Plugin for AmbientCGPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource::<AmbientCGPath>(AmbientCGPath(self.base_path.clone()));
    }
}

#[derive(Clone, Resource)]
pub struct AmbientCGPath(String);

#[allow(dead_code)]
#[derive(Default)]
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

#[derive(Default)]
pub struct AmbientCGMaterial<'a> {
    pub name: &'a str,
    pub resolution: AmbientCGResolution,
    pub subfolder: Option<&'a str>,
    pub uv_scale: Option<Vec2>,
}

impl<'a> AmbientCGMaterial<'a> {
    pub fn load(
        &self,
        base_path: AmbientCGPath,
        asset_server: Res<'_, AssetServer>,
        materials: &mut ResMut<'_, Assets<StandardMaterial>>
    ) -> Handle<StandardMaterial> {
        let mut material_path = PathBuf::from_str(&base_path.0).unwrap();

        if let Some(subfolder) = &self.subfolder {
            material_path.push(subfolder);
        }

        let constructed_material_name = format!("{}_{}-JPG", self.name, self.resolution);
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
                if let Some(uv_scale) = self.uv_scale {
                    return Affine2::from_scale(uv_scale);
                }
                Affine2::default()
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