use crate::core::*;
use crate::renderer::*;
use std::rc::Rc;

pub enum TerrainLod {
    Standard,
    Coarse,
    VeryCoarse,
}

pub struct Terrain<M: Material> {
    context: Context,
    center: (i32, i32),
    patches: Vec<Gm<TerrainPatch, M>>,
    index_buffer: Rc<ElementBuffer>,
    coarse_index_buffer: Rc<ElementBuffer>,
    very_coarse_index_buffer: Rc<ElementBuffer>,
    material: M,
    lod: Box<dyn Fn(f32) -> TerrainLod>,
    height_map: Box<dyn Fn(f32, f32) -> f32>,
    patch_size: f32,
    patches_per_side: u32,
}
impl<M: Material + Clone> Terrain<M> {
    pub fn new(
        context: &Context,
        material: M,
        height_map: Box<dyn Fn(f32, f32) -> f32>,
        patch_size: f32,
        patches_per_side: u32,
        initial_position: Vec3,
    ) -> Self {
        let index_buffer = Rc::new(ElementBuffer::new_with_data(context, &Self::indices(1)));
        let mut patches = Vec::new();
        let center = Self::pos2patch(patch_size, initial_position);
        let half_patches_per_side = Self::half_patches_per_side(patches_per_side);
        for ix in center.0 - half_patches_per_side..center.0 + half_patches_per_side + 1 {
            for iy in center.1 - half_patches_per_side..center.1 + half_patches_per_side + 1 {
                let patch = TerrainPatch::new(
                    context,
                    &height_map,
                    (ix, iy),
                    patch_size,
                    index_buffer.clone(),
                );
                patches.push(Gm::new(patch, material.clone()));
            }
        }
        Self {
            context: context.clone(),
            center,
            patches,
            index_buffer,
            coarse_index_buffer: Rc::new(ElementBuffer::new_with_data(context, &Self::indices(4))),
            very_coarse_index_buffer: Rc::new(ElementBuffer::new_with_data(
                context,
                &Self::indices(8),
            )),
            lod: Box::new(|_| TerrainLod::Standard),
            material: material.clone(),
            height_map,
            patch_size,
            patches_per_side,
        }
    }

    pub fn set_lod(&mut self, lod: Box<dyn Fn(f32) -> TerrainLod>) {
        self.lod = lod;
    }

    pub fn update(&mut self, position: Vec3) {
        let (x0, y0) = Self::pos2patch(self.patch_size, position);
        let half_patches_per_side = Self::half_patches_per_side(self.patches_per_side);

        while x0 > self.center.0 {
            self.center.0 += 1;
            for iy in
                self.center.1 - half_patches_per_side..self.center.1 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (self.center.0 + half_patches_per_side, iy),
                        self.patch_size,
                        self.index_buffer.clone(),
                    ),
                    self.material.clone(),
                ));
            }
        }

        while x0 < self.center.0 {
            self.center.0 -= 1;
            for iy in
                self.center.1 - half_patches_per_side..self.center.1 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (self.center.0 - half_patches_per_side, iy),
                        self.patch_size,
                        self.index_buffer.clone(),
                    ),
                    self.material.clone(),
                ));
            }
        }
        while y0 > self.center.1 {
            self.center.1 += 1;
            for ix in
                self.center.0 - half_patches_per_side..self.center.0 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (ix, self.center.1 + half_patches_per_side),
                        self.patch_size,
                        self.index_buffer.clone(),
                    ),
                    self.material.clone(),
                ));
            }
        }

        while y0 < self.center.1 {
            self.center.1 -= 1;
            for ix in
                self.center.0 - half_patches_per_side..self.center.0 + half_patches_per_side + 1
            {
                self.patches.push(Gm::new(
                    TerrainPatch::new(
                        &self.context,
                        &self.height_map,
                        (ix, self.center.1 - half_patches_per_side),
                        self.patch_size,
                        self.index_buffer.clone(),
                    ),
                    self.material.clone(),
                ));
            }
        }

        self.patches.retain(|p| {
            let (ix, iy) = p.index();
            (x0 - ix).abs() <= half_patches_per_side && (y0 - iy).abs() <= half_patches_per_side
        });

        self.patches.iter_mut().for_each(|p| {
            let distance = p.center().distance(vec3(position.x, 0.0, position.z));
            p.index_buffer = match (*self.lod)(distance) {
                TerrainLod::VeryCoarse => self.very_coarse_index_buffer.clone(),
                TerrainLod::Coarse => self.coarse_index_buffer.clone(),
                TerrainLod::Standard => self.index_buffer.clone(),
            };
        })
    }

    fn indices(resolution: u32) -> Vec<u32> {
        let mut indices: Vec<u32> = Vec::new();
        let stride = VERTICES_PER_SIDE as u32;
        let max = (stride - 1) / resolution;
        for r in 0..max {
            for c in 0..max {
                indices.push(r * resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + (c * resolution + resolution) * stride);
            }
        }
        indices
    }

    fn half_patches_per_side(patches_per_side: u32) -> i32 {
        (patches_per_side as i32 - 1) / 2
    }

    fn pos2patch(patch_size: f32, position: Vec3) -> (i32, i32) {
        (
            (position.x / patch_size).floor() as i32,
            (position.z / patch_size).floor() as i32,
        )
    }

    ///
    /// Returns an iterator over the reference to the objects in this terrain which can be used as input to a render function, for example [RenderTarget::render].
    ///
    pub fn obj_iter(&self) -> impl Iterator<Item = &dyn Object> + Clone {
        self.patches.iter().map(|m| m as &dyn Object)
    }

    ///
    /// Returns an iterator over the reference to the geometries in this terrain which can be used as input to for example [pick], [RenderTarget::render_with_material] or [DirectionalLight::generate_shadow_map].
    ///
    pub fn geo_iter(&self) -> impl Iterator<Item = &dyn Geometry> + Clone {
        self.patches.iter().map(|m| m as &dyn Geometry)
    }
}