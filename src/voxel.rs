/// Celda de la grilla. `material_id == 0` representa aire (celda vacia);
/// cualquier otro valor indexa la tabla de materiales.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Voxel {
    pub material_id: u8,
}

impl Voxel {
    pub const EMPTY: Voxel = Voxel { material_id: 0 };

    pub fn new(material_id: u8) -> Self {
        Voxel { material_id }
    }

    pub fn occupied(&self) -> bool {
        self.material_id != 0
    }
}
