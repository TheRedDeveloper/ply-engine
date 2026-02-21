use std::borrow::Cow;

/// Represents a shader asset that can be loaded from a file path, embedded as source,
/// or looked up from the runtime shader storage in [`MaterialManager`].
///
/// `Path` is loaded from the filesystem at runtime (useful for development/hot-reloading).
/// `Source` embeds the shader in the binary (via `include_str!`).
/// `Stored` references a named entry in the shader storage, enabling runtime-updateable shader code.
#[derive(Debug, Clone)]
pub enum ShaderAsset {
    /// Path to a compiled .glsl file, loaded at runtime
    Path(&'static str),
    /// Embedded GLSL ES 3.00 fragment shader source
    Source {
        /// Cache key for MaterialManager
        file_name: &'static str,
        /// GLSL ES 3.00 fragment shader source
        fragment: &'static str,
    },
    /// References a named entry in the runtime shader storage.
    Stored(&'static str),
}

impl ShaderAsset {
    /// Returns the fragment shader source.
    /// For `Path` variant, reads the file synchronously.
    /// For `Source` variant, returns a borrowed reference (zero-copy).
    /// For `Stored` variant, looks up the source from the global shader storage.
    pub fn fragment_source(&self) -> Cow<'static, str> {
        match self {
            ShaderAsset::Path(path) => {
                Cow::Owned(std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("Failed to read shader file '{}': {}", path, e)))
            }
            ShaderAsset::Source { fragment, .. } => Cow::Borrowed(fragment),
            ShaderAsset::Stored(name) => {
                let mgr = crate::renderer::MATERIAL_MANAGER.lock().unwrap();
                match mgr.get_source(name) {
                    Some(src) => Cow::Owned(src.to_string()),
                    None => {
                        eprintln!("Shader storage '{}' not found, using fallback", name);
                        Cow::Borrowed(crate::renderer::DEFAULT_FRAGMENT_SHADER)
                    }
                }
            }
        }
    }

    /// Returns the cache key used by MaterialManager.
    pub fn cache_key(&self) -> &str {
        match self {
            ShaderAsset::Path(path) => path,
            ShaderAsset::Source { file_name, .. } => file_name,
            ShaderAsset::Stored(name) => name,
        }
    }
}

/// Configuration for a shader effect, stored in render commands.
/// Contains the fragment shader source and uniform values.
#[derive(Debug, Clone)]
pub struct ShaderConfig {
    /// The GLSL ES 3.00 fragment shader source (resolved from ShaderAsset).
    pub fragment: Cow<'static, str>,
    /// The uniform values to set on the shader.
    pub uniforms: Vec<ShaderUniform>,
    /// Debug-friendly name derived from the shader asset.
    pub name: String,
}

/// A single shader uniform with a name and typed value.
#[derive(Debug, Clone)]
pub struct ShaderUniform {
    /// The uniform variable name in the shader.
    pub name: String,
    /// The value to set for this uniform.
    pub value: ShaderUniformValue,
}

/// Typed values for shader uniforms.
#[derive(Debug, Clone)]
pub enum ShaderUniformValue {
    /// A single float value.
    Float(f32),
    /// A 2-component float vector.
    Vec2([f32; 2]),
    /// A 3-component float vector.
    Vec3([f32; 3]),
    /// A 4-component float vector.
    Vec4([f32; 4]),
    /// A single integer value.
    Int(i32),
    /// A 4x4 matrix.
    Mat4([[f32; 4]; 4]),
}

impl From<f32> for ShaderUniformValue {
    fn from(v: f32) -> Self {
        ShaderUniformValue::Float(v)
    }
}

impl From<[f32; 2]> for ShaderUniformValue {
    fn from(v: [f32; 2]) -> Self {
        ShaderUniformValue::Vec2(v)
    }
}

impl From<[f32; 3]> for ShaderUniformValue {
    fn from(v: [f32; 3]) -> Self {
        ShaderUniformValue::Vec3(v)
    }
}

impl From<[f32; 4]> for ShaderUniformValue {
    fn from(v: [f32; 4]) -> Self {
        ShaderUniformValue::Vec4(v)
    }
}

impl From<i32> for ShaderUniformValue {
    fn from(v: i32) -> Self {
        ShaderUniformValue::Int(v)
    }
}

impl From<[[f32; 4]; 4]> for ShaderUniformValue {
    fn from(v: [[f32; 4]; 4]) -> Self {
        ShaderUniformValue::Mat4(v)
    }
}

/// Builder for configuring shader uniforms.
/// Used in the closure passed to `.effect()` and `.shader()` on `ElementBuilder`.
pub struct ShaderBuilder<'a> {
    source: &'a ShaderAsset,
    uniforms: Vec<ShaderUniform>,
}

impl<'a> ShaderBuilder<'a> {
    /// Creates a new ShaderBuilder for the given shader asset.
    pub(crate) fn new(source: &'a ShaderAsset) -> Self {
        Self {
            source,
            uniforms: Vec::new(),
        }
    }

    /// Sets a uniform value on the shader.
    ///
    /// Supports `f32`, `[f32; 2]`, `[f32; 3]`, `[f32; 4]`, `i32`, and `[[f32; 4]; 4]`.
    pub fn uniform(&mut self, name: &str, value: impl Into<ShaderUniformValue>) -> &mut Self {
        self.uniforms.push(ShaderUniform {
            name: name.to_string(),
            value: value.into(),
        });
        self
    }

    /// Builds the ShaderConfig from this builder.
    pub(crate) fn into_config(&mut self) -> ShaderConfig {
        ShaderConfig {
            fragment: self.source.fragment_source(),
            uniforms: std::mem::take(&mut self.uniforms),
            name: self.source.cache_key().to_string(),
        }
    }
}
