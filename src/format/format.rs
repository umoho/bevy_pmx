#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxTextEncoding {
    Utf8,
    Utf16Le,
}

impl Default for PmxTextEncoding {
    fn default() -> Self {
        Self::Utf8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxBoneFlags(pub u16);

impl PmxBoneFlags {
    pub const TAIL_BONE: u16 = 0x0001;
    pub const ROTATABLE: u16 = 0x0002;
    pub const TRANSLATABLE: u16 = 0x0004;
    pub const VISIBLE: u16 = 0x0008;
    pub const ENABLED: u16 = 0x0010;
    pub const IK: u16 = 0x0020;
    pub const INHERIT_ROTATION: u16 = 0x0100;
    pub const INHERIT_TRANSLATION: u16 = 0x0200;
    pub const FIXED_AXIS: u16 = 0x0400;
    pub const LOCAL_AXES: u16 = 0x0800;
    pub const AFTER_PHYSICS: u16 = 0x1000;
    pub const EXTERNAL_PARENT_DEFORM: u16 = 0x2000;

    pub fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxMaterialFlags(pub u8);

impl PmxMaterialFlags {
    pub const NO_CULL: u8 = 0x01;
    pub const GROUND_SHADOW: u8 = 0x02;
    pub const DRAW_SHADOW: u8 = 0x04;
    pub const RECEIVE_SHADOW: u8 = 0x08;
    pub const HAS_EDGE: u8 = 0x10;
    pub const VERTEX_COLOR: u8 = 0x20;
    pub const POINT_DRAWING: u8 = 0x40;
    pub const LINE_DRAWING: u8 = 0x80;

    pub fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxHeader {
    pub magic: [u8; 4],
    pub version: f32,
    pub header_size: u8,
    pub encoding: PmxTextEncoding,
    pub additional_uv_count: u8,
    pub vertex_index_size: u8,
    pub texture_index_size: u8,
    pub material_index_size: u8,
    pub bone_index_size: u8,
    pub morph_index_size: u8,
    pub rigid_body_index_size: u8,
    pub model_name: String,
    pub model_name_english: String,
    pub comment: String,
    pub comment_english: String,
}

impl Default for PmxHeader {
    fn default() -> Self {
        Self {
            magic: *b"PMX ",
            version: 2.0,
            header_size: 8,
            encoding: PmxTextEncoding::default(),
            additional_uv_count: 0,
            vertex_index_size: 4,
            texture_index_size: 4,
            material_index_size: 4,
            bone_index_size: 4,
            morph_index_size: 4,
            rigid_body_index_size: 4,
            model_name: String::new(),
            model_name_english: String::new(),
            comment: String::new(),
            comment_english: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxDocument {
    pub header: PmxHeader,
    pub vertices: Vec<PmxVertex>,
    pub indices: Vec<u32>,
    pub textures: Vec<PmxTexture>,
    pub materials: Vec<PmxMaterial>,
    pub bones: Vec<PmxBone>,
    pub morphs: Vec<PmxMorph>,
    pub display_frames: Vec<PmxDisplayFrame>,
    pub rigid_bodies: Vec<PmxRigidBody>,
    pub joints: Vec<PmxJoint>,
    pub soft_bodies: Vec<PmxSoftBody>,
}

impl Default for PmxDocument {
    fn default() -> Self {
        Self {
            header: PmxHeader::default(),
            vertices: Vec::new(),
            indices: Vec::new(),
            textures: Vec::new(),
            materials: Vec::new(),
            bones: Vec::new(),
            morphs: Vec::new(),
            display_frames: Vec::new(),
            rigid_bodies: Vec::new(),
            joints: Vec::new(),
            soft_bodies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub additional_uvs: Vec<[f32; 4]>,
    pub weight: PmxVertexWeight,
    pub edge_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PmxVertexWeight {
    Bdef1 {
        bone: i32,
    },
    Bdef2 {
        bone0: i32,
        bone1: i32,
        weight: f32,
    },
    Bdef4 {
        bones: [i32; 4],
        weights: [f32; 4],
    },
    Sdef {
        bone0: i32,
        bone1: i32,
        weight: f32,
        c: [f32; 3],
        r0: [f32; 3],
        r1: [f32; 3],
    },
    Qdef {
        bones: [i32; 4],
        weights: [f32; 4],
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxTexture {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxMaterial {
    pub name: String,
    pub name_english: String,
    pub diffuse: [f32; 4],
    pub specular: [f32; 3],
    pub specular_strength: f32,
    pub ambient: [f32; 3],
    pub flags: PmxMaterialFlags,
    pub edge_color: [f32; 4],
    pub edge_size: f32,
    pub texture_index: i32,
    pub sphere_texture_index: i32,
    pub sphere_mode: PmxSphereMode,
    pub toon_sharing: bool,
    pub toon_texture_index: i32,
    pub comment: String,
    /// Number of vertex indices contributed by this material.
    ///
    /// PMX stores material coverage as a triangle-index count, not as a primitive count.
    pub surface_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxSphereMode {
    Disabled,
    Multiply,
    Add,
    SubTexture,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxBone {
    pub name: String,
    pub name_english: String,
    pub position: [f32; 3],
    pub parent_bone: i32,
    pub layer: i32,
    pub flags: PmxBoneFlags,
    pub tail: PmxBoneTail,
    pub inheritance: Option<PmxBoneInheritance>,
    pub fixed_axis: Option<[f32; 3]>,
    pub local_axes: Option<PmxBoneAxes>,
    pub external_parent: i32,
    pub ik: Option<PmxIk>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PmxBoneTail {
    BoneIndex(i32),
    Offset([f32; 3]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxBoneInheritance {
    pub parent_bone: i32,
    pub influence: f32,
    pub affects_translation: bool,
    pub affects_rotation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxBoneAxes {
    pub local_x: [f32; 3],
    pub local_z: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxIk {
    pub target_bone: i32,
    pub iterations: u32,
    pub limit_radians: f32,
    pub links: Vec<PmxIkLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxIkLink {
    pub bone_index: i32,
    pub angle_limits: Option<([f32; 3], [f32; 3])>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxMorph {
    pub name: String,
    pub name_english: String,
    pub panel: PmxMorphPanel,
    pub kind: PmxMorphKind,
    pub offsets: Vec<PmxMorphOffset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxMorphPanel {
    System,
    Eyebrow,
    Eye,
    Mouth,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxMorphKind {
    Group,
    Vertex,
    Bone,
    Uv,
    AdditionalUv(usize),
    Material,
    Flip,
    Impulse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PmxMorphOffset {
    Group {
        morph_index: i32,
        influence: f32,
    },
    Vertex {
        vertex_index: i32,
        offset: [f32; 3],
    },
    Bone {
        bone_index: i32,
        translation: [f32; 3],
        rotation: [f32; 4],
    },
    Uv {
        vertex_index: i32,
        offset: [f32; 4],
    },
    Material {
        material_index: i32,
        operation: PmxMaterialMorphOperation,
        morph: PmxMaterialMorph,
    },
    Flip {
        morph_index: i32,
        influence: f32,
    },
    Impulse {
        rigid_body_index: i32,
        local: bool,
        velocity: [f32; 3],
        angular_velocity: [f32; 3],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxMaterialMorphOperation {
    Multiply,
    Add,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxMaterialMorph {
    pub diffuse: [f32; 4],
    pub specular: [f32; 3],
    pub specular_strength: f32,
    pub ambient: [f32; 3],
    pub edge_color: [f32; 4],
    pub edge_size: f32,
    pub texture_tint: [f32; 4],
    pub sphere_tint: [f32; 4],
    pub toon_tint: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxDisplayFrame {
    pub name: String,
    pub name_english: String,
    pub special: bool,
    pub items: Vec<PmxDisplayFrameItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PmxDisplayFrameItem {
    Bone(i32),
    Morph(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxRigidBody {
    pub name: String,
    pub name_english: String,
    pub bone_index: i32,
    pub group: u8,
    pub mask: u16,
    pub shape: PmxRigidBodyShape,
    pub size: [f32; 3],
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub mass: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub restitution: f32,
    pub friction: f32,
    pub mode: PmxRigidBodyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxRigidBodyShape {
    Sphere,
    Box,
    Capsule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxRigidBodyMode {
    FollowBone,
    Physics,
    PhysicsAndBone,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxJoint {
    pub name: String,
    pub name_english: String,
    pub joint_type: PmxJointKind,
    pub body_a: i32,
    pub body_b: i32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub translation_limit_min: [f32; 3],
    pub translation_limit_max: [f32; 3],
    pub rotation_limit_min: [f32; 3],
    pub rotation_limit_max: [f32; 3],
    pub spring_translation: [f32; 3],
    pub spring_rotation: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxJointKind {
    Spring6Dof,
    SixDof,
    P2p,
    ConeTwist,
    Slider,
    Hinge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PmxSoftBody {
    pub name: String,
    pub name_english: String,
    pub shape: PmxSoftBodyShape,
    pub material_index: i32,
    pub group: u8,
    pub mask: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxSoftBodyShape {
    TriMesh,
    Rope,
}
