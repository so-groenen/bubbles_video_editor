use opencv::{core::Size, core::RotateFlags};
use std::ffi::OsString;

pub trait OpenCvRotationCode
{
    fn code(&self) -> i32;    
}
impl OpenCvRotationCode for RotateFlags
{
    fn code(&self) -> i32
    {
        <RotateFlags as Into<i32>>::into(*self)
    }
}

pub fn get_video_name(file_path: &std::path::PathBuf, default: &str) -> String
{
    let default   = OsString::from(default);
    let file_name = file_path.file_name()
        .unwrap_or(&default)
        .to_str()
        .expect("Failed converting OsString to &str");
    String::from(file_name)
}

pub trait SizeEdit
{
    fn resize(&mut self, scale: Option<f32>) -> &mut Self;
    fn get_resized(&self, scale: f32) -> Self;
    fn rotate(&mut self, rotation: Option<RotateFlags>) -> &mut Self;
}
impl SizeEdit for opencv::core::Size
{
    fn resize(&mut self, scale: Option<f32>) -> &mut Self
    {
        if let Some(scale) = scale
        {
            self.height = (self.height as f32 * scale) as i32;
            self.width  =  (self.width as f32 * scale) as i32;
        }
        self
    }    
    fn get_resized(&self, scale: f32) -> Self 
    {
        let mut new_size = Size {width: self.width, height: self.height};
        new_size.resize(Some(scale));
        new_size
    }
    fn rotate(&mut self, flip: Option<RotateFlags>) ->  &mut Self 
    {
        if let Some(rotation) = flip
        {
            match rotation
            {
                RotateFlags::ROTATE_180 => (),
                _ =>
                {
                    let temp = self.width;
                    self.width = self.height;
                    self.height = temp;
                },
            }
        }
        self
    }
}
// fourcc = Four Character Code (ex: "DivX", "Xvid", "mp4a")
// fourcc example [12345678][09876543][32745186][62137854]
// The get the first byte (= [62137854]), we need to "AND' it with "[00000000][00000000][00000000][11111111]"
// which is "0xFF"
// To get the second byte (=[32745186]) we byte shift by 8 bits to the right then AND it:
// example >> 8 = [00000000][12345678][09876543][32745186]
// THEN:
//     [00000000][12345678][09876543][32745186]
// AND [00000000][00000000][00000000][11111111]
// =   [32745186]
// etc ...


pub fn decode_fourcc(fourcc_codec: u32) -> Option<(char, char, char, char)>
{
    if fourcc_codec <= 0
    {
        return None    
    }

    let byte_shift_0: usize = 0;
    let byte_shift_1: usize = 1*8;
    let byte_shift_2: usize = 2*8;
    let byte_shift_3: usize = 3*8;

    let byte0 = ((fourcc_codec >> byte_shift_0) & 0xFF) as u8;
    let byte1 = ((fourcc_codec >> byte_shift_1) & 0xFF) as u8;
    let byte2 = ((fourcc_codec >> byte_shift_2) & 0xFF) as u8;
    let byte3 = ((fourcc_codec >> byte_shift_3) & 0xFF) as u8;

    let video_codec = (byte0 as char, byte1 as char, byte2 as char, byte3 as char); 

    return Some(video_codec);
}
