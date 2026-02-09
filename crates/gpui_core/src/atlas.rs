use crate::{Bounds, DevicePixels};

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct AtlasTile {
    pub texture_id: AtlasTextureId,
    pub tile_id: TileId,
    pub padding: u32,
    pub bounds: Bounds<DevicePixels>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct AtlasTextureId {
    // We use u32 instead of usize for Metal Shader Language compatibility
    pub index: u32,
    pub kind: AtlasTextureKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum AtlasTextureKind {
    Monochrome = 0,
    Polychrome = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct TileId(pub u32);

impl From<etagere::AllocId> for TileId {
    fn from(id: etagere::AllocId) -> Self {
        Self(id.serialize())
    }
}

impl From<TileId> for etagere::AllocId {
    fn from(id: TileId) -> Self {
        Self::deserialize(id.0)
    }
}
