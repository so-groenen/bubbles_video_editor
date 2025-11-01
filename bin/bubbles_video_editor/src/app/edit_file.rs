use std::ffi::OsStr;

#[derive(Default)]
pub struct EditFile 
{
    edit_file_buffer: String,
    edit_file_path: std::path::PathBuf,
    edit_file_name: String,
}
impl EditFile
{
    const DEFAULT_FILENAME: &str = "edit.mp4";
    fn create_default_edit_path(file_name: &std::path::PathBuf, placer_holder: &str) -> std::path::PathBuf
    {
        let default_directory = match std::env::current_dir()
        {
            Ok(cwd) => cwd,
            Err(_)  => std::path::PathBuf::from(""),
        };

        let parent    = file_name.parent().unwrap_or(&default_directory);
        let extension = file_name.extension().unwrap_or(OsStr::new("no_extension"));
        let mut new_file_stem = file_name.file_stem().unwrap_or(OsStr::new("empty_file_name")).to_owned();

        println!("DEBUG: Filename {}", new_file_stem.to_str().expect("String not empty"));
        println!("DEBUG: extension {}", extension.to_str().expect("String not empty"));
        new_file_stem.push(placer_holder);
        
        let mut processed_file_path = parent.join(new_file_stem);
        processed_file_path.set_extension(extension);
        processed_file_path
    }
    pub fn new(file_path: &std::path::PathBuf, placer_holder: &str) -> Self
    {
        let edit_file_path   = EditFile::create_default_edit_path(&file_path, placer_holder);
        let edit_file_buffer = String::from(edit_file_path.to_str().expect("edit_file_buffer: Could not Path to &str."));
        let edit_file_name   = edit_file_path.file_name().unwrap_or(OsStr::new(EditFile::DEFAULT_FILENAME)).display().to_string();
        Self { edit_file_buffer, edit_file_path, edit_file_name }
    }   
    pub fn get_buffer(&mut self) -> &mut String
    {
        &mut self.edit_file_buffer
    }
    pub fn get_path(&mut self) -> &std::path::PathBuf
    {
        &self.edit_file_path
    }
    pub fn get_name(&mut self) -> &String
    {
        &self.edit_file_name
    }
    pub fn update_from_buffer(&mut self)
    {                    
        self.edit_file_path   = std::path::PathBuf::from(&self.edit_file_buffer);
        self.edit_file_name   = self.edit_file_path.file_name().unwrap_or(OsStr::new(EditFile::DEFAULT_FILENAME)).display().to_string();
    }
    pub fn update_from_path(&mut self, path: std::path::PathBuf)
    {
        self.edit_file_buffer = path.display().to_string();
        self.edit_file_path   = path;
        self.edit_file_name   = self.edit_file_path.file_name().unwrap_or(OsStr::new(EditFile::DEFAULT_FILENAME)).display().to_string();
    }
}
 