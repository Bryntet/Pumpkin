use super::registry_getter::RegistryGetter;
use std::error::Error;
use std::path::Path;
use std::{fs, io};
use tempfile::TempDir;

pub(crate) struct JarGetter<'a> {
    version: &'a str,
    registry_getters: Vec<Box<dyn RegistryGetter>>,
}

impl<'a> JarGetter<'a> {
    pub fn new(version: &'a str, registry_getters: Vec<Box<dyn RegistryGetter>>) -> JarGetter<'a> {
        Self {
            version,
            registry_getters,
        }
    }

    fn get_link() -> Result<String, Box<dyn Error>> {
        let response = reqwest::blocking::get("https://www.minecraft.net/en-us/download/server")?;
        let text = response.text()?;

        let out = text
            .lines()
            .find(|s| s.contains("piston-data.mojang.com"))
            .unwrap()
            .split('"')
            .nth(1)
            .unwrap()
            .to_string();

        Ok(out)
    }

    fn needs_update(&self) -> bool {
        fs::File::open(self.file_path()).is_err()
    }

    pub(crate) fn check_update(&self) -> Result<(), Box<dyn Error>> {
        if !Path::new("../assets-build").exists() {
            fs::create_dir("../assets-build")?;
        }
        if self.needs_update() {
            for file in fs::read_dir("../assets-build")?.flatten() {
                if file.file_type()?.is_dir() {
                    fs::remove_dir_all(file.path())?;
                } else {
                    fs::remove_file(file.path())?;
                }
            }
            self.download_file()?;
            dbg!("past that");
            let data = self.get_data()?;
            dbg!("past again");
            for registry_getter in &self.registry_getters {
                registry_getter.create_file(data.path())?;
            }
            data.close()?;
        }
        Ok(())
    }

    fn file_path(&self) -> String {
        format!("../assets-build/{}.jar", self.version)
    }

    fn download_file(&self) -> Result<(), Box<dyn Error>> {
        let response = reqwest::blocking::get(Self::get_link()?)?;
        let mut file = fs::File::create(self.file_path())?;
        let content = response.bytes()?;
        io::copy(&mut content.as_ref(), &mut file)?;
        Ok(())
    }

    fn get_data(&self) -> Result<TempDir, Box<dyn Error>> {
        let temp_dir = tempfile::tempdir()?;

        let mut archive = zip::ZipArchive::new(self.get_inner_zip()?)?;

        archive.extract(temp_dir.path())?;
        let mut dir = temp_dir.path().to_path_buf();
        dir.push("data");
        dir.push("minecraft");

        let data = tempfile::tempdir()?;

        for file in dir.read_dir()?.flatten() {
            let path = data.as_ref().join(file.file_name());
            fs::create_dir(&path)?;
            copy_dir_all(&file.path(), &path)?;
        }

        temp_dir.close()?;

        Ok(data)
    }

    fn get_inner_zip(&self) -> Result<fs::File, Box<dyn Error>> {
        let file = fs::File::open(self.file_path())?;
        let mut archive = zip::ZipArchive::new(file)?;

        let path = format!(
            "META-INF/versions/{}/server-{}.jar",
            self.version, self.version
        );
        let mut inner_zip = archive.by_name(&path).unwrap();
        let mut temp_file = tempfile::tempfile()?;
        io::copy(&mut inner_zip, &mut temp_file)?;
        Ok(temp_file)
    }
}
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::get_data::JarGetter;
    use crate::registry_getter::RecipeGetter;
    use std::fs;
    use std::path::Path;

    #[test]
    fn read_website() {
        if Path::new("../assets-build").exists() {
            fs::remove_dir_all("../assets-build").unwrap();
        }
        JarGetter::new("1.21.3", vec![Box::new(RecipeGetter)])
            .check_update()
            .unwrap();
    }
}
