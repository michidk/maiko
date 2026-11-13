use crate::{
    build::assembly,
    finish::finish,
    image::Image,
    kit::{Kit, Target},
    prepare::{Files, prepare},
};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

fn error(message: String) -> JsValue {
    JsValue::from_str(&message)
}
fn to_js(files: Files) -> Result<JsValue, JsValue> {
    let object = Object::new();
    for (name, bytes) in files {
        Reflect::set(&object, &name.into(), &Uint8Array::from(bytes.as_slice()))?;
    }
    Ok(object.into())
}
fn from_js(value: JsValue) -> Result<Files, JsValue> {
    if !value.is_object() {
        return Err(error("Expected compiler files".into()));
    }
    let mut files = Files::new();
    for key in Object::keys(&Object::from(value.clone())) {
        let name = key
            .as_string()
            .ok_or_else(|| error("Invalid compiler filename".into()))?;
        if !crate::kit::safe_path(&name) {
            return Err(error("Unsafe compiler filename".into()));
        }
        let bytes = Reflect::get(&value, &key)?;
        if !bytes.is_instance_of::<Uint8Array>() {
            return Err(error("Expected byte arrays".into()));
        }
        files.insert(name, Uint8Array::new(&bytes).to_vec());
    }
    Ok(files)
}

#[wasm_bindgen]
pub struct Builder {
    input: Vec<u8>,
    kit: Kit,
}
#[wasm_bindgen]
impl Builder {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &[u8], kit: &[u8], target: &str) -> Result<Builder, JsValue> {
        Image::load(input).map_err(error)?;
        let kit = Kit::load(kit, Target::parse(target).map_err(error)?).map_err(error)?;
        Ok(Self {
            input: input.to_vec(),
            kit,
        })
    }
    pub fn prepare(&self) -> Result<JsValue, JsValue> {
        let mut files = self.kit.compiler_files();
        files.extend(prepare(&self.input).map_err(error)?);
        to_js(files)
    }
    pub fn finish(&self, files: JsValue) -> Result<JsValue, JsValue> {
        to_js(finish(&self.input, from_js(files)?).map_err(error)?)
    }
    pub fn tool(&self, name: &str) -> Result<Vec<u8>, JsValue> {
        if !["sr", "nasm"].contains(&name) {
            return Err(error("Unknown compiler tool".into()));
        }
        self.kit
            .files
            .get(&format!("tools/{name}.wasm"))
            .cloned()
            .ok_or_else(|| error("Missing compiler tool".into()))
    }
    pub fn assembly(&self) -> Array {
        assembly(self.kit.manifest.target)
            .into_iter()
            .map(|(args, output)| {
                let command = Array::new();
                command.push(&args.into_iter().map(JsValue::from).collect::<Array>());
                command.push(&output.into());
                JsValue::from(command)
            })
            .collect()
    }
    pub fn relink(&self, assembled: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.kit.relink(assembled).map_err(error)
    }
    pub fn executable(&self) -> String {
        self.kit.manifest.target.executable().into()
    }
    pub fn package(&self, executable: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.kit.package(executable).map_err(error)
    }
}

#[wasm_bindgen]
pub fn validate(input: &[u8]) -> Result<String, JsValue> {
    Image::load(input)
        .map(|image| image.source_hash)
        .map_err(error)
}
