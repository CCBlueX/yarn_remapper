# Yarn Remapper

`yarn_remapper` is a Rust library that remaps Minecraft Yarn named mappings to their obfuscated counterparts. It parses the TINY v2 mapping format provided by FabricMC and enables the remapping of class names, method names, field names, and their descriptors. This tool is essential for accessing obfuscated classes, fields, and methods via the Java Native Interface (JNI) and is a foundational component of [LiquidBounce Lite](https://github.com/CCBlueX/liquidbounce_lite), a Minecraft DLL injection client written in Rust.

## How it Works

The `yarn_remapper` library leverages the TINY v2 mapping format, which uses hierarchical sections to define mappings between named, intermediary, and official obfuscated class names, method names, field names, and descriptors. These mappings help transform names from the readable Yarn mappings to the obfuscated names used in Minecraft's official releases.

Example of a TINY v2 format snippet:
```plaintext
tiny	2	0	official	intermediary	named
c	a	class_123	pkg/SomeClass
	f	[I	a	field_789	someField
	m	(III)V	a	method_456	someMethod
		p	1		param_0	x
		p	2		param_1	y
		p	3		param_2	z
c	b	class_234	pkg/xy/AnotherClass
	m	(Ljava/lang/String;)I	a	method_567	anotherMethod
```

## Installation
Add yarn_remapper to your Cargo.toml dependencies:

```toml
[dependencies]
yarn_remapper = "0.1.1"
```
Ensure you have downloaded the mapping file required for remapping:

TINY v2 Mapping File: [yarn-1.20.4-rc1+build.1-mergedv2.jar](https://maven.fabricmc.net/net/fabricmc/yarn/1.20.4%2Bbuild.1/yarn-1.20.4%2Bbuild.1-mergedv2.jar)

## Usage
Here's an example of how to use yarn_remapper in your Rust project:

```rust
use yarn_remapper::{Mapping};
use yarn_remapper::mapping::MappingLoader;
use yarn_remapper::tiny_v2::TinyV2Mapping;
use std::path::Path;

fn main() -> Result<(), Error> {
    // Path to the TINY v2 mapping file
    let path = Path::new("path/to/mappings.tiny");
    
    // Parse mappings
    let mapping = TinyV2Mapping::load(&path)?;

    // Remap a class name
    if let Some(obfuscated_name) = mapping.remap_class("net/minecraft/client/MinecraftClient") {
        println!("Obfuscated class name: {}", obfuscated_name);
    }

    // Remap a method name with descriptor
    if let Some(obfuscated_method_name) = mapping.remap_method("net/minecraft/client/MinecraftClient", "getWindowTitle", "()Ljava/lang/String;") {
        println!("Obfuscated method name: {}", obfuscated_method_name);
    }

    // Remap a field name with descriptor
    if let Some(obfuscated_field_name) = mapping.remap_field("net/minecraft/client/MinecraftClient", "inGameHud", "Lnet/minecraft/client/gui/hud/InGameHud;") {
        println!("Obfuscated field name: {}", obfuscated_field_name);
    }

    Ok(())
}
```

## License
This project is licensed under the GNU GPLv3 License - see the LICENSE file for details.

## Contributions
Contributions are welcome! Please open a pull request or an issue to contribute to the project or suggest improvements.

> This project is not affiliated with Mojang or Minecraft.