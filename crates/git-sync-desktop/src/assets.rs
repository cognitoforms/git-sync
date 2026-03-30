use std::borrow::Cow;

use gpui::*;

/// Asset source combining our custom icons with the built-in ones from `gpui-component`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let base_assets = gpui_component_assets::Assets;
        let custom_assets = crate::ui::CustomAssets;

        let result = match (base_assets.load(path), custom_assets.load(path)) {
            (Ok(base), Ok(custom)) => base.or(custom),
            (Ok(some), Err(_)) | (Err(_), Ok(some)) => some,
            (Err(e1), Err(_)) => {
                return Err(e1);
            }
        };

        Ok(result)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let base_assets = gpui_component_assets::Assets;
        let custom_assets = crate::ui::CustomAssets;

        let mut assets = base_assets.list(path)?;
        assets.append(&mut custom_assets.list(path)?);
        Ok(assets)
    }
}
