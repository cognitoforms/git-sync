use std::borrow::Cow;

use anyhow::Result;
use gpui_component::IconNamed;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct CustomAssets;

impl gpui::AssetSource for CustomAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Self::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

pub enum CustomIcon {
    RotateCCW,
}

impl IconNamed for CustomIcon {
    fn path(self) -> gpui::SharedString {
        match self {
            Self::RotateCCW => "icons/rotate-ccw.svg".into(),
        }
    }
}
