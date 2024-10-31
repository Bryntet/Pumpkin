use crate::registry_getter::RecipeGetter;
use std::error::Error;

mod get_data;
mod registry_getter;

pub fn check_that_latest_version_is_achieved(version: &str) -> Result<(), Box<dyn Error>> {
    get_data::JarGetter::new(version, vec![Box::new(RecipeGetter)]).check_update()
}
