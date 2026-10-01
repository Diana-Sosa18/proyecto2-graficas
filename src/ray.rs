use nalgebra_glm::Vec3;

/// Rayo parametrico: P(t) = origen + t * direccion (direccion normalizada).
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Ray { origin, dir: dir.normalize() }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}
