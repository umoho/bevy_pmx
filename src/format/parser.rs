use std::{
    convert::TryFrom,
    io::{Cursor, Read},
};

use crate::error::{PmxError, PmxResult};

use super::format::*;

impl PmxDocument {
    pub fn from_bytes(bytes: &[u8]) -> PmxResult<Self> {
        Self::read_from(&mut Cursor::new(bytes))
    }

    pub fn read_from<R: Read + ?Sized>(reader: &mut R) -> PmxResult<Self> {
        PmxParser::new(reader).parse_document()
    }
}

pub fn parse_pmx(bytes: &[u8]) -> PmxResult<PmxDocument> {
    PmxDocument::from_bytes(bytes)
}

struct PmxParser<'a, R: Read + ?Sized> {
    reader: &'a mut R,
}

impl<'a, R: Read + ?Sized> PmxParser<'a, R> {
    fn new(reader: &'a mut R) -> Self {
        Self { reader }
    }

    fn parse_document(&mut self) -> PmxResult<PmxDocument> {
        let header = self.read_header()?;
        let vertices = self.read_vertices(&header)?;
        let indices = self.read_indices(&header)?;
        let textures = self.read_textures(&header)?;
        let materials = self.read_materials(&header)?;
        let bones = self.read_bones(&header)?;
        let morphs = self.read_morphs(&header)?;
        let display_frames = self.read_display_frames(&header)?;
        let rigid_bodies = self.read_rigid_bodies(&header)?;
        let joints = self.read_joints(&header)?;
        let soft_bodies = if header.version >= 2.1 {
            self.read_soft_bodies(&header)?
        } else {
            Vec::new()
        };

        Ok(PmxDocument {
            header,
            vertices,
            indices,
            textures,
            materials,
            bones,
            morphs,
            display_frames,
            rigid_bodies,
            joints,
            soft_bodies,
        })
    }

    fn read_header(&mut self) -> PmxResult<PmxHeader> {
        let magic = self.read_array::<4>()?;
        if magic != *b"PMX " {
            return Err(PmxError::InvalidMagic(magic));
        }

        let version = self.read_f32()?;
        if version != 2.0 && version != 2.1 {
            return Err(PmxError::UnsupportedVersion(version));
        }

        let header_size = self.read_u8()?;
        if header_size < 8 {
            return Err(PmxError::InvalidFormat(
                "pmx header size must be at least 8 bytes",
            ));
        }

        let encoding = match self.read_u8()? {
            0 => PmxTextEncoding::Utf16Le,
            1 => PmxTextEncoding::Utf8,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX text encoding")),
        };
        let additional_uv_count = self.read_u8()?;
        let vertex_index_size = self.read_u8()?;
        let texture_index_size = self.read_u8()?;
        let material_index_size = self.read_u8()?;
        let bone_index_size = self.read_u8()?;
        let morph_index_size = self.read_u8()?;
        let rigid_body_index_size = self.read_u8()?;

        if header_size > 8 {
            self.skip(usize::from(header_size - 8))?;
        }

        let model_name = self.read_text(encoding)?;
        let model_name_english = self.read_text(encoding)?;
        let comment = self.read_text(encoding)?;
        let comment_english = self.read_text(encoding)?;

        Ok(PmxHeader {
            magic,
            version,
            header_size,
            encoding,
            additional_uv_count,
            vertex_index_size,
            texture_index_size,
            material_index_size,
            bone_index_size,
            morph_index_size,
            rigid_body_index_size,
            model_name,
            model_name_english,
            comment,
            comment_english,
        })
    }

    fn read_vertices(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxVertex>> {
        let count = self.read_len()?;
        let mut vertices = Vec::with_capacity(count);
        for _ in 0..count {
            vertices.push(self.read_vertex(header)?);
        }
        Ok(vertices)
    }

    fn read_vertex(&mut self, header: &PmxHeader) -> PmxResult<PmxVertex> {
        let position = self.read_vec3()?;
        let normal = self.read_vec3()?;
        let uv = self.read_vec2()?;
        let mut additional_uvs = Vec::with_capacity(usize::from(header.additional_uv_count));
        for _ in 0..header.additional_uv_count {
            additional_uvs.push(self.read_vec4()?);
        }

        let weight = match self.read_u8()? {
            0 => PmxVertexWeight::Bdef1 {
                bone: self.read_index(header.bone_index_size)?,
            },
            1 => PmxVertexWeight::Bdef2 {
                bone0: self.read_index(header.bone_index_size)?,
                bone1: self.read_index(header.bone_index_size)?,
                weight: self.read_f32()?,
            },
            2 => {
                let mut bones = [0; 4];
                for bone in &mut bones {
                    *bone = self.read_index(header.bone_index_size)?;
                }

                let mut weights = [0.0; 4];
                for weight in &mut weights {
                    *weight = self.read_f32()?;
                }

                PmxVertexWeight::Bdef4 { bones, weights }
            }
            3 => PmxVertexWeight::Sdef {
                bone0: self.read_index(header.bone_index_size)?,
                bone1: self.read_index(header.bone_index_size)?,
                weight: self.read_f32()?,
                c: self.read_vec3()?,
                r0: self.read_vec3()?,
                r1: self.read_vec3()?,
            },
            4 => {
                let mut bones = [0; 4];
                for bone in &mut bones {
                    *bone = self.read_index(header.bone_index_size)?;
                }

                let mut weights = [0.0; 4];
                for weight in &mut weights {
                    *weight = self.read_f32()?;
                }

                PmxVertexWeight::Qdef { bones, weights }
            }
            _ => {
                return Err(PmxError::InvalidFormat(
                    "unsupported PMX vertex weight type",
                ));
            }
        };

        let edge_scale = self.read_f32()?;

        Ok(PmxVertex {
            position,
            normal,
            uv,
            additional_uvs,
            weight,
            edge_scale,
        })
    }

    fn read_indices(&mut self, header: &PmxHeader) -> PmxResult<Vec<u32>> {
        let count = self.read_len()?;
        let mut indices = Vec::with_capacity(count);
        for _ in 0..count {
            indices.push(self.read_vertex_index(header.vertex_index_size)?);
        }
        Ok(indices)
    }

    fn read_textures(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxTexture>> {
        let count = self.read_len()?;
        let mut textures = Vec::with_capacity(count);
        for _ in 0..count {
            textures.push(PmxTexture {
                path: self.read_text(header.encoding)?,
            });
        }
        Ok(textures)
    }

    fn read_materials(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxMaterial>> {
        let count = self.read_len()?;
        let mut materials = Vec::with_capacity(count);
        for _ in 0..count {
            materials.push(self.read_material(header)?);
        }
        Ok(materials)
    }

    fn read_material(&mut self, header: &PmxHeader) -> PmxResult<PmxMaterial> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let diffuse = self.read_vec4()?;
        let specular = self.read_vec3()?;
        let specular_strength = self.read_f32()?;
        let ambient = self.read_vec3()?;
        let flags = PmxMaterialFlags(self.read_u8()?);
        let edge_color = self.read_vec4()?;
        let edge_size = self.read_f32()?;
        let texture_index = self.read_index(header.texture_index_size)?;
        let sphere_texture_index = self.read_index(header.texture_index_size)?;
        let sphere_mode = match self.read_u8()? {
            0 => PmxSphereMode::Disabled,
            1 => PmxSphereMode::Multiply,
            2 => PmxSphereMode::Add,
            3 => PmxSphereMode::SubTexture,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX sphere mode")),
        };
        let toon_sharing = self.read_u8()? != 0;
        let toon_texture_index = if toon_sharing {
            i32::from(self.read_u8()?)
        } else {
            self.read_index(header.texture_index_size)?
        };
        let comment = self.read_text(header.encoding)?;
        let surface_count = u32::try_from(self.read_len()?)
            .map_err(|_| PmxError::InvalidFormat("surface count is too large"))?;

        Ok(PmxMaterial {
            name,
            name_english,
            diffuse,
            specular,
            specular_strength,
            ambient,
            flags,
            edge_color,
            edge_size,
            texture_index,
            sphere_texture_index,
            sphere_mode,
            toon_sharing,
            toon_texture_index,
            comment,
            surface_count,
        })
    }

    fn read_bones(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxBone>> {
        let count = self.read_len()?;
        let mut bones = Vec::with_capacity(count);
        for _ in 0..count {
            bones.push(self.read_bone(header)?);
        }
        Ok(bones)
    }

    fn read_bone(&mut self, header: &PmxHeader) -> PmxResult<PmxBone> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let position = self.read_vec3()?;
        let parent_bone = self.read_index(header.bone_index_size)?;
        let layer = self.read_i32()?;
        let flags = PmxBoneFlags(self.read_u16()?);
        let tail = if flags.contains(PmxBoneFlags::TAIL_BONE) {
            PmxBoneTail::BoneIndex(self.read_index(header.bone_index_size)?)
        } else {
            PmxBoneTail::Offset(self.read_vec3()?)
        };

        let inheritance = if flags.contains(PmxBoneFlags::INHERIT_ROTATION)
            || flags.contains(PmxBoneFlags::INHERIT_TRANSLATION)
        {
            Some(PmxBoneInheritance {
                parent_bone: self.read_index(header.bone_index_size)?,
                influence: self.read_f32()?,
                affects_translation: flags.contains(PmxBoneFlags::INHERIT_TRANSLATION),
                affects_rotation: flags.contains(PmxBoneFlags::INHERIT_ROTATION),
            })
        } else {
            None
        };

        let fixed_axis = if flags.contains(PmxBoneFlags::FIXED_AXIS) {
            Some(self.read_vec3()?)
        } else {
            None
        };

        let local_axes = if flags.contains(PmxBoneFlags::LOCAL_AXES) {
            Some(PmxBoneAxes {
                local_x: self.read_vec3()?,
                local_z: self.read_vec3()?,
            })
        } else {
            None
        };

        let external_parent = if flags.contains(PmxBoneFlags::EXTERNAL_PARENT_DEFORM) {
            self.read_i32()?
        } else {
            -1
        };

        let ik = if flags.contains(PmxBoneFlags::IK) {
            Some(self.read_ik(header)?)
        } else {
            None
        };

        Ok(PmxBone {
            name,
            name_english,
            position,
            parent_bone,
            layer,
            flags,
            tail,
            inheritance,
            fixed_axis,
            local_axes,
            external_parent,
            ik,
        })
    }

    fn read_ik(&mut self, header: &PmxHeader) -> PmxResult<PmxIk> {
        let target_bone = self.read_index(header.bone_index_size)?;
        let iterations = self.read_len()?;
        let limit_radians = self.read_f32()?;
        let link_count = self.read_len()?;
        let mut links = Vec::with_capacity(link_count);
        for _ in 0..link_count {
            let bone_index = self.read_index(header.bone_index_size)?;
            let angle_limits = if self.read_u8()? != 0 {
                Some((self.read_vec3()?, self.read_vec3()?))
            } else {
                None
            };
            links.push(PmxIkLink {
                bone_index,
                angle_limits,
            });
        }

        Ok(PmxIk {
            target_bone,
            iterations: u32::try_from(iterations)
                .map_err(|_| PmxError::InvalidFormat("IK iteration count is too large"))?,
            limit_radians,
            links,
        })
    }

    fn read_morphs(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxMorph>> {
        let count = self.read_len()?;
        let mut morphs = Vec::with_capacity(count);
        for _ in 0..count {
            morphs.push(self.read_morph(header)?);
        }
        Ok(morphs)
    }

    fn read_morph(&mut self, header: &PmxHeader) -> PmxResult<PmxMorph> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let panel = match self.read_u8()? {
            0 => PmxMorphPanel::System,
            1 => PmxMorphPanel::Eyebrow,
            2 => PmxMorphPanel::Eye,
            3 => PmxMorphPanel::Mouth,
            4 => PmxMorphPanel::Other,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX morph panel")),
        };
        let kind_byte = self.read_u8()?;
        let kind = match kind_byte {
            0 => PmxMorphKind::Group,
            1 => PmxMorphKind::Vertex,
            2 => PmxMorphKind::Bone,
            3 => PmxMorphKind::Uv,
            4..=7 => PmxMorphKind::AdditionalUv(usize::from(kind_byte - 3)),
            8 => PmxMorphKind::Material,
            9 => PmxMorphKind::Flip,
            10 => PmxMorphKind::Impulse,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX morph kind")),
        };
        let offset_count = self.read_len()?;
        let mut offsets = Vec::with_capacity(offset_count);
        for _ in 0..offset_count {
            offsets.push(self.read_morph_offset(header, kind)?);
        }

        Ok(PmxMorph {
            name,
            name_english,
            panel,
            kind,
            offsets,
        })
    }

    fn read_morph_offset(
        &mut self,
        header: &PmxHeader,
        kind: PmxMorphKind,
    ) -> PmxResult<PmxMorphOffset> {
        match kind {
            PmxMorphKind::Group => Ok(PmxMorphOffset::Group {
                morph_index: self.read_index(header.morph_index_size)?,
                influence: self.read_f32()?,
            }),
            PmxMorphKind::Vertex => Ok(PmxMorphOffset::Vertex {
                vertex_index: self.read_index(header.vertex_index_size)?,
                offset: self.read_vec3()?,
            }),
            PmxMorphKind::Bone => Ok(PmxMorphOffset::Bone {
                bone_index: self.read_index(header.bone_index_size)?,
                translation: self.read_vec3()?,
                rotation: self.read_vec4()?,
            }),
            PmxMorphKind::Uv | PmxMorphKind::AdditionalUv(_) => Ok(PmxMorphOffset::Uv {
                vertex_index: self.read_index(header.vertex_index_size)?,
                offset: self.read_vec4()?,
            }),
            PmxMorphKind::Material => Ok(PmxMorphOffset::Material {
                material_index: self.read_index(header.material_index_size)?,
                operation: match self.read_u8()? {
                    0 => PmxMaterialMorphOperation::Multiply,
                    1 => PmxMaterialMorphOperation::Add,
                    _ => {
                        return Err(PmxError::InvalidFormat(
                            "unsupported PMX material morph operation",
                        ));
                    }
                },
                morph: self.read_material_morph()?,
            }),
            PmxMorphKind::Flip => Ok(PmxMorphOffset::Flip {
                morph_index: self.read_index(header.morph_index_size)?,
                influence: self.read_f32()?,
            }),
            PmxMorphKind::Impulse => Ok(PmxMorphOffset::Impulse {
                rigid_body_index: self.read_index(header.rigid_body_index_size)?,
                local: self.read_u8()? != 0,
                velocity: self.read_vec3()?,
                angular_velocity: self.read_vec3()?,
            }),
        }
    }

    fn read_material_morph(&mut self) -> PmxResult<PmxMaterialMorph> {
        Ok(PmxMaterialMorph {
            diffuse: self.read_vec4()?,
            specular: self.read_vec3()?,
            specular_strength: self.read_f32()?,
            ambient: self.read_vec3()?,
            edge_color: self.read_vec4()?,
            edge_size: self.read_f32()?,
            texture_tint: self.read_vec4()?,
            sphere_tint: self.read_vec4()?,
            toon_tint: self.read_vec4()?,
        })
    }

    fn read_display_frames(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxDisplayFrame>> {
        let count = self.read_len()?;
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            frames.push(self.read_display_frame(header)?);
        }
        Ok(frames)
    }

    fn read_display_frame(&mut self, header: &PmxHeader) -> PmxResult<PmxDisplayFrame> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let special = self.read_u8()? != 0;
        let item_count = self.read_len()?;
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            items.push(match self.read_u8()? {
                0 => PmxDisplayFrameItem::Bone(self.read_index(header.bone_index_size)?),
                1 => PmxDisplayFrameItem::Morph(self.read_index(header.morph_index_size)?),
                _ => {
                    return Err(PmxError::InvalidFormat(
                        "unsupported PMX display frame item",
                    ));
                }
            });
        }

        Ok(PmxDisplayFrame {
            name,
            name_english,
            special,
            items,
        })
    }

    fn read_rigid_bodies(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxRigidBody>> {
        let count = self.read_len()?;
        let mut rigid_bodies = Vec::with_capacity(count);
        for _ in 0..count {
            rigid_bodies.push(self.read_rigid_body(header)?);
        }
        Ok(rigid_bodies)
    }

    fn read_rigid_body(&mut self, header: &PmxHeader) -> PmxResult<PmxRigidBody> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let bone_index = self.read_index(header.bone_index_size)?;
        let group = self.read_u8()?;
        let mask = self.read_u16()?;
        let shape = match self.read_u8()? {
            0 => PmxRigidBodyShape::Sphere,
            1 => PmxRigidBodyShape::Box,
            2 => PmxRigidBodyShape::Capsule,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX rigid body shape")),
        };
        let size = self.read_vec3()?;
        let position = self.read_vec3()?;
        let rotation = self.read_vec3()?;
        let mass = self.read_f32()?;
        let linear_damping = self.read_f32()?;
        let angular_damping = self.read_f32()?;
        let restitution = self.read_f32()?;
        let friction = self.read_f32()?;
        let mode = match self.read_u8()? {
            0 => PmxRigidBodyMode::FollowBone,
            1 => PmxRigidBodyMode::Physics,
            2 => PmxRigidBodyMode::PhysicsAndBone,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX rigid body mode")),
        };

        Ok(PmxRigidBody {
            name,
            name_english,
            bone_index,
            group,
            mask,
            shape,
            size,
            position,
            rotation,
            mass,
            linear_damping,
            angular_damping,
            restitution,
            friction,
            mode,
        })
    }

    fn read_joints(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxJoint>> {
        let count = self.read_len()?;
        let mut joints = Vec::with_capacity(count);
        for _ in 0..count {
            joints.push(self.read_joint(header)?);
        }
        Ok(joints)
    }

    fn read_joint(&mut self, header: &PmxHeader) -> PmxResult<PmxJoint> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let joint_type = match self.read_u8()? {
            0 => PmxJointKind::Spring6Dof,
            1 => PmxJointKind::SixDof,
            2 => PmxJointKind::P2p,
            3 => PmxJointKind::ConeTwist,
            4 => PmxJointKind::Slider,
            5 => PmxJointKind::Hinge,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX joint kind")),
        };
        let body_a = self.read_index(header.rigid_body_index_size)?;
        let body_b = self.read_index(header.rigid_body_index_size)?;
        let position = self.read_vec3()?;
        let rotation = self.read_vec3()?;
        let translation_limit_min = self.read_vec3()?;
        let translation_limit_max = self.read_vec3()?;
        let rotation_limit_min = self.read_vec3()?;
        let rotation_limit_max = self.read_vec3()?;
        let spring_translation = self.read_vec3()?;
        let spring_rotation = self.read_vec3()?;

        Ok(PmxJoint {
            name,
            name_english,
            joint_type,
            body_a,
            body_b,
            position,
            rotation,
            translation_limit_min,
            translation_limit_max,
            rotation_limit_min,
            rotation_limit_max,
            spring_translation,
            spring_rotation,
        })
    }

    fn read_soft_bodies(&mut self, header: &PmxHeader) -> PmxResult<Vec<PmxSoftBody>> {
        let count = self.read_len()?;
        let mut soft_bodies = Vec::with_capacity(count);
        for _ in 0..count {
            soft_bodies.push(self.read_soft_body(header)?);
        }
        Ok(soft_bodies)
    }

    fn read_soft_body(&mut self, header: &PmxHeader) -> PmxResult<PmxSoftBody> {
        let name = self.read_text(header.encoding)?;
        let name_english = self.read_text(header.encoding)?;
        let shape = match self.read_u8()? {
            0 => PmxSoftBodyShape::TriMesh,
            1 => PmxSoftBodyShape::Rope,
            _ => return Err(PmxError::InvalidFormat("unsupported PMX soft body shape")),
        };
        let material_index = self.read_index(header.material_index_size)?;
        let group = self.read_u8()?;
        let mask = self.read_u16()?;

        self.read_u8()?; // flags
        self.read_i32()?; // b_link_create_distance
        self.read_i32()?; // number of clusters
        self.read_f32()?; // total mass
        self.read_f32()?; // collision margin
        self.read_i32()?; // aerodynamics model
        for _ in 0..12 {
            self.read_f32()?;
        }
        for _ in 0..6 {
            self.read_f32()?;
        }
        for _ in 0..4 {
            self.read_i32()?;
        }
        for _ in 0..3 {
            self.read_f32()?;
        }

        let anchor_count = self.read_len()?;
        for _ in 0..anchor_count {
            self.read_index(header.rigid_body_index_size)?;
            self.read_vertex_index(header.vertex_index_size)?;
            self.read_u8()?; // near mode
        }

        let pin_count = self.read_len()?;
        for _ in 0..pin_count {
            self.read_vertex_index(header.vertex_index_size)?;
        }

        Ok(PmxSoftBody {
            name,
            name_english,
            shape,
            material_index,
            group,
            mask,
        })
    }

    fn read_text(&mut self, encoding: PmxTextEncoding) -> PmxResult<String> {
        let len = self.read_len()?;
        let bytes = self.read_bytes(len)?;
        match encoding {
            PmxTextEncoding::Utf8 => {
                String::from_utf8(bytes).map_err(|_| PmxError::InvalidFormat("invalid UTF-8 text"))
            }
            PmxTextEncoding::Utf16Le => {
                if bytes.len() % 2 != 0 {
                    return Err(PmxError::InvalidFormat(
                        "UTF-16LE text byte length must be even",
                    ));
                }

                let mut units = Vec::with_capacity(bytes.len() / 2);
                for chunk in bytes.chunks_exact(2) {
                    units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
                }

                String::from_utf16(&units)
                    .map_err(|_| PmxError::InvalidFormat("invalid UTF-16LE text"))
            }
        }
    }

    fn read_len(&mut self) -> PmxResult<usize> {
        let len = self.read_i32()?;
        usize::try_from(len).map_err(|_| PmxError::InvalidFormat("negative PMX length"))
    }

    fn read_vertex_index(&mut self, size: u8) -> PmxResult<u32> {
        let index = self.read_index(size)?;
        u32::try_from(index).map_err(|_| PmxError::InvalidFormat("vertex index cannot be negative"))
    }

    fn read_index(&mut self, size: u8) -> PmxResult<i32> {
        match size {
            1 => Ok(i32::from(self.read_i8()?)),
            2 => Ok(i32::from(self.read_i16()?)),
            4 => self.read_i32(),
            _ => Err(PmxError::InvalidFormat("unsupported PMX index width")),
        }
    }

    fn read_vec2(&mut self) -> PmxResult<[f32; 2]> {
        Ok([self.read_f32()?, self.read_f32()?])
    }

    fn read_vec3(&mut self) -> PmxResult<[f32; 3]> {
        Ok([self.read_f32()?, self.read_f32()?, self.read_f32()?])
    }

    fn read_vec4(&mut self) -> PmxResult<[f32; 4]> {
        Ok([
            self.read_f32()?,
            self.read_f32()?,
            self.read_f32()?,
            self.read_f32()?,
        ])
    }

    fn read_u8(&mut self) -> PmxResult<u8> {
        let mut bytes = [0u8; 1];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes[0])
    }

    fn read_i8(&mut self) -> PmxResult<i8> {
        Ok(i8::from_le_bytes([self.read_u8()?]))
    }

    fn read_u16(&mut self) -> PmxResult<u16> {
        let mut bytes = [0u8; 2];
        self.reader.read_exact(&mut bytes)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_i16(&mut self) -> PmxResult<i16> {
        let mut bytes = [0u8; 2];
        self.reader.read_exact(&mut bytes)?;
        Ok(i16::from_le_bytes(bytes))
    }

    fn read_i32(&mut self) -> PmxResult<i32> {
        let mut bytes = [0u8; 4];
        self.reader.read_exact(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }

    fn read_f32(&mut self) -> PmxResult<f32> {
        let mut bytes = [0u8; 4];
        self.reader.read_exact(&mut bytes)?;
        Ok(f32::from_le_bytes(bytes))
    }

    fn read_array<const N: usize>(&mut self) -> PmxResult<[u8; N]> {
        let mut bytes = [0u8; N];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn read_bytes(&mut self, len: usize) -> PmxResult<Vec<u8>> {
        let mut bytes = vec![0u8; len];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn skip(&mut self, len: usize) -> PmxResult<()> {
        let mut buffer = [0u8; 64];
        let mut remaining = len;
        while remaining > 0 {
            let chunk = remaining.min(buffer.len());
            self.reader.read_exact(&mut buffer[..chunk])?;
            remaining -= chunk;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_pmx;

    fn push_f32(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_i32(bytes: &mut Vec<u8>, value: i32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn parses_minimal_pmx_20_document() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PMX ");
        push_f32(&mut bytes, 2.0);
        bytes.push(8);
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&[4, 4, 4, 4, 4, 4]);

        for _ in 0..4 {
            push_i32(&mut bytes, 0);
        }

        for _ in 0..9 {
            push_i32(&mut bytes, 0);
        }

        let document = parse_pmx(&bytes).expect("minimal PMX should parse");
        assert_eq!(document.header.magic, *b"PMX ");
        assert_eq!(document.header.version, 2.0);
        assert_eq!(document.header.header_size, 8);
        assert!(document.vertices.is_empty());
        assert!(document.indices.is_empty());
        assert!(document.textures.is_empty());
        assert!(document.materials.is_empty());
        assert!(document.bones.is_empty());
        assert!(document.morphs.is_empty());
        assert!(document.display_frames.is_empty());
        assert!(document.rigid_bodies.is_empty());
        assert!(document.joints.is_empty());
        assert!(document.soft_bodies.is_empty());
    }
}
