# `bevy_pmx`

为 Bevy 提供 PMX 文件读取支持

## TODO

- [x] (阶段 0) 结构
- [x] (阶段 0) 解析器
- [x] (阶段 0) 文件夹源
- [x] (阶段 0) 纹理解析
- [x] (阶段 0) Primitive
- [x] (阶段 0) 加载器
- [x] (阶段 0) 插件
- [x] (阶段 0) 原文档
- [x] (阶段 0) 纹理路径
- [x] (阶段 0) BMP/TGA
- [x] (阶段 0) 源感知加载
- [x] (阶段 1) 原文档保留
- [x] (阶段 1) Mesh
- [x] (阶段 1) 材质
- [x] (阶段 1) 子资产
- [x] (阶段 2) 骨骼
- [x] (阶段 2) 层级
- [x] (阶段 2) Morph
- [x] (阶段 2) 物理
- [x] (阶段 3) Zip
- [x] (阶段 3) 样例
- [ ] (阶段 3) 测试
- [ ] (阶段 3) 文档

## 用法

### AssetServer

注册插件后，可以像普通资产一样加载 `.pmx` 文件：

```rust
use bevy::prelude::*;
use bevy_pmx::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PmxPlugin::default())
        .run();
}

#[derive(Resource)]
struct ModelHandle(Handle<Pmx>);

fn load_model(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<Pmx> = asset_server.load("models/character.pmx");
    commands.insert_resource(ModelHandle(handle));
}
```

加载完成后，可以从 `Assets<Pmx>` 读取：

```rust
fn use_model(pmx_assets: Res<Assets<Pmx>>, handle: Res<ModelHandle>) {
    let Some(model) = pmx_assets.get(&handle.0) else {
        return;
    };

    let _geometry = model.geometry();
    let _materials = model.material_records();
    let _bones = model.bone_records();
    let _morphs = model.morph_records();
}
```

### 手动导入

如果你需要自己控制文件来源，或者要从 `.zip` 中读取 PMX 和纹理，可以直接使用 `PmxSource` 和 `import_pmx`。`.zip` 支持需要开启 `zip` feature：

```rust
use bevy_pmx::prelude::*;

let source = PmxSource::folder("assets/models/character");
// 或者：
// 需要开启 `zip` feature
// let source = PmxSource::zip("assets/models/character.zip", "character");

let bytes = source.read_bytes(source.resolve("character.pmx"))?;
let document = parse_pmx(&bytes)?;
let model = import_pmx(document, &PmxImportContext::with_source(source)).model;
```

## 示例

```bash
cargo run --example pmx_viewer -- path/to/model.pmx
cargo run --features zip --example pmx_viewer -- path/to/model.zip
```

## 版本

| bevy | bevy_pmx |
|------|----------|
| 0.19 | 0.2      |
| 0.18 | 0.1      |
