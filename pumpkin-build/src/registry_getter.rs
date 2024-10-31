use serde_json::Value;
use std::error::Error;
use std::fs;
use std::path::Path;

pub trait RegistryGetter {
    fn output_path(&self) -> &'static str;
    fn build_registry_file(&self, top_dir_path: &Path) -> Result<Value, Box<dyn Error>>;

    fn create_file(&self, top_dir_path: &Path) -> Result<(), Box<dyn Error>> {
        let value = self.build_registry_file(top_dir_path)?;
        let path = format!("../assets-build/{}.json", self.output_path());
        fs::File::create(&path)?;
        fs::write(&path, value.to_string())?;
        Ok(())
    }
}

pub struct RecipeGetter;

impl RegistryGetter for RecipeGetter {
    fn output_path(&self) -> &'static str {
        "recipes"
    }

    fn build_registry_file(&self, top_dir_path: &Path) -> Result<Value, Box<dyn Error>> {
        let mut path = top_dir_path.to_path_buf();
        path.push("recipe");
        dbg!(&path);
        let mut map: serde_json::Map<String, Value> = serde_json::Map::new();
        for recipe in path.read_dir()?.flatten() {
            let recipe_name = recipe
                .file_name()
                .to_str()
                .unwrap()
                .strip_suffix(".json")
                .unwrap()
                .to_string();
            let value: Value = serde_json::from_slice(&fs::read(recipe.path())?)?;
            map.insert(recipe_name.to_string(), value);
        }
        Ok(Value::Object(map))
    }
}
